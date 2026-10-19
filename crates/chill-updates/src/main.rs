use chill_updates::{
    AntiRollbackStore, ApplyRequest, Component, ExtractionLimits, FIXED_GITHUB_OWNER,
    FIXED_GITHUB_REPOSITORY, GithubReleaseClient, InstallHooks, InstallLayout, IntentKind,
    OperationStatus, PolicyMode, RuntimeCompatibility, StateStore, TransportLimits, UpdateError,
    VerificationContext, apply_staged, authorize_activation, due_attempts_for,
    parse_and_verify_manifest, parse_strict_json, record_retry, recover_interrupted,
    validate_root_owned_directory, validate_root_owned_file, verify_staged_artifact,
};
use reqwest::redirect::Policy as RedirectPolicy;
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::thread;
use std::time::{Duration, Instant, UNIX_EPOCH};
use time::{OffsetDateTime, UtcOffset};
use uuid::Uuid;

const DEFAULT_CONFIG: &str = "/etc/chillcenter/updates.json";
const CONFIRM_INTERRUPT: &str = "INTERRUPT_ACTIVE_WORK";

/// Serializes network/cache/install mutations across timer and administrator
/// invocations. Recovery intentionally does not acquire this lock: it runs as
/// service pre-start and must be able to repair a killed updater.
struct HelperLease(File);
impl HelperLease {
    fn acquire(config: &RootConfig) -> chill_updates::Result<Self> {
        let path = config
            .maintenance_flag
            .parent()
            .ok_or_else(|| UpdateError::Security("runtime directory missing".into()))?
            .join("helper.lock");
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path)?;
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(UpdateError::Busy(
                "another updater invocation is active".into(),
            ));
        }
        Ok(Self(file))
    }
}
impl Drop for HelperLease {
    fn drop(&mut self) {
        unsafe {
            libc::flock(self.0.as_raw_fd(), libc::LOCK_UN);
        }
    }
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RootConfig {
    schema_version: u32,
    key_id: String,
    public_key: PathBuf,
    state_root: PathBuf,
    trust_root: PathBuf,
    cache_root: PathBuf,
    backup_root: PathBuf,
    maintenance_flag: PathBuf,
    quiescence_status: PathBuf,
    server_config: PathBuf,
    service_name: String,
    service_uid: u32,
    service_gid: u32,
    health_url: String,
    status_url: String,
    #[serde(default)]
    health_resolve: Option<String>,
    health_ca_certificate: PathBuf,
    component_roots: BTreeMap<Component, PathBuf>,
    targets: BTreeMap<Component, String>,
    bootstrap_versions: BTreeMap<Component, Version>,
    api_version: Version,
    database_schema: u32,
    renderer_version: u32,
    theme_format: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct QuiescenceStatus {
    quiescing: bool,
    busy: bool,
    active_mutations: u32,
    updated_unix: i64,
    maintenance_generation_ns: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AvailableReceipt {
    schema_version: u32,
    operation_id: Uuid,
    component: Component,
    target: String,
    release_tag: String,
    cache_key: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AvailabilityEntry {
    version: Version,
    source_commit: String,
    release_sequence: u64,
    checked_unix: i64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BackupManifest {
    format_version: u32,
    schema_version: u32,
    data_root: PathBuf,
    entries: Vec<BackupEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BackupEntry {
    path: String,
    size: u64,
    sha256: String,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("chill-update: {error}");
        std::process::exit(2);
    }
}

fn run() -> chill_updates::Result<()> {
    let mut arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments == ["--help"] || arguments.is_empty() {
        print_help();
        return Ok(());
    }
    if arguments == ["--version"] {
        println!("chill-update {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let config_path = if arguments.first().map(String::as_str) == Some("--config") {
        if arguments.len() < 3 {
            return Err(UpdateError::Invalid(
                "--config requires an absolute path and command".into(),
            ));
        }
        let path = PathBuf::from(arguments.remove(1));
        arguments.remove(0);
        path
    } else {
        PathBuf::from(DEFAULT_CONFIG)
    };
    let recovery_only = matches!(
        arguments.first().map(String::as_str),
        Some("recover" | "rescue")
    );
    let config = load_config(&config_path, recovery_only)?;
    let mut _lease = None;
    if matches!(
        arguments.first().map(String::as_str),
        Some("check" | "apply" | "install" | "tick" | "rescue")
    ) {
        _lease = Some(HelperLease::acquire(&config)?);
    }
    match arguments.first().map(String::as_str) {
        Some("inspect") if arguments.len() == 1 => inspect(&config),
        Some("check") if arguments.len() == 2 => {
            perform_check(&config, parse_component(&arguments[1])?).map(|_| ())
        }
        Some("manifest") if arguments.len() == 2 => {
            verify_cached(&config, parse_component(&arguments[1])?).map(|_| ())
        }
        Some("recover") if arguments.len() == 2 => {
            let component = parse_component(&arguments[1])?;
            require_linux_component(component)?;
            let root = config
                .component_roots
                .get(&component)
                .ok_or_else(|| UpdateError::Security("component root missing".into()))?;
            println!(
                "{:?}",
                recover_interrupted(root, Some(&activation_authorization(&config, component)))?
            );
            Ok(())
        }
        Some("rescue") if arguments.len() == 1 => rescue(&config),
        Some("apply") | Some("install") => {
            if arguments.len() < 2 {
                return Err(UpdateError::Invalid("component is required".into()));
            }
            let component = parse_component(&arguments[1])?;
            let first_install = arguments[0] == "install";
            let force = arguments.iter().any(|arg| arg == "--force");
            let retry_accepted = arguments.iter().any(|arg| arg == "--retry-accepted");
            let confirmed = arguments
                .iter()
                .any(|arg| arg == &format!("--confirm-interrupt={CONFIRM_INTERRUPT}"));
            if force && !confirmed {
                return Err(UpdateError::Security(format!(
                    "--force requires --confirm-interrupt={CONFIRM_INTERRUPT}"
                )));
            }
            if arguments.iter().skip(2).any(|arg| {
                arg != "--force"
                    && arg != "--retry-accepted"
                    && !arg.starts_with("--confirm-interrupt=")
            }) {
                return Err(UpdateError::Invalid("unknown apply option".into()));
            }
            perform_apply(
                &config,
                component,
                first_install,
                force,
                retry_accepted,
                false,
            )
        }
        Some("policy") if arguments.len() == 6 => {
            let component = parse_component(&arguments[1])?;
            let mode = match arguments[2].as_str() {
                "manual" => PolicyMode::Manual,
                "check" => PolicyMode::Check,
                "automatic" => PolicyMode::Automatic,
                _ => return Err(UpdateError::Invalid("invalid policy mode".into())),
            };
            let hour = arguments[3]
                .parse()
                .map_err(|_| UpdateError::Invalid("invalid hour".into()))?;
            let minute = arguments[4]
                .parse()
                .map_err(|_| UpdateError::Invalid("invalid minute".into()))?;
            let revision = arguments[5]
                .parse()
                .map_err(|_| UpdateError::Invalid("invalid expected revision".into()))?;
            let updated = state_store(&config)?
                .compare_and_set_policy(component, revision, mode, hour, minute)?;
            println!("policy revision {}", updated.revision);
            Ok(())
        }
        Some("intent") if arguments.len() == 3 => {
            let component = parse_component(&arguments[1])?;
            let kind = match arguments[2].as_str() {
                "check" => IntentKind::Check,
                "install" => IntentKind::Install,
                _ => return Err(UpdateError::Invalid("invalid intent".into())),
            };
            let operation =
                state_store(&config)?.admit_intent(component, kind, OffsetDateTime::now_utc())?;
            println!("{}", operation.id);
            Ok(())
        }
        Some("tick") if arguments.len() == 1 => tick(&config),
        _ => Err(UpdateError::Invalid("invalid command; use --help".into())),
    }
}

fn print_help() {
    println!(
        "chill-update {}\n\nUsage:\n  chill-update [--config PATH] inspect\n  chill-update [--config PATH] check COMPONENT\n  chill-update [--config PATH] manifest COMPONENT\n  chill-update [--config PATH] policy COMPONENT MODE HOUR MINUTE EXPECTED_REVISION\n  chill-update [--config PATH] intent COMPONENT check|install\n  chill-update [--config PATH] tick\n  chill-update [--config PATH] recover COMPONENT\n  chill-update [--config PATH] apply COMPONENT [--retry-accepted] [--force --confirm-interrupt={CONFIRM_INTERRUPT}]\n  chill-update [--config PATH] install COMPONENT\n\nCOMPONENT is server, web, or themes on Linux. `install` is first-install only.\n`--retry-accepted` is local root-only recovery and is never admitted by the LAN API.\nNo command accepts a URL, repository, key, shell command, tag, or destination path.",
        env!("CARGO_PKG_VERSION")
    );
}

fn load_config(path: &Path, recovery_only: bool) -> chill_updates::Result<RootConfig> {
    validate_root_owned_file(path, 128 * 1024)?;
    let bytes = read_bounded(path, 128 * 1024)?;
    let config: RootConfig = parse_strict_json(&bytes)?;
    if config.schema_version != 1
        || config.key_id.is_empty()
        || config.key_id.len() > 128
        || config.service_name != "chillcenter.service"
        || config.component_roots.len() != 3
        || config.targets.len() != 3
    {
        return Err(UpdateError::Security(
            "root config schema or fixed values are invalid".into(),
        ));
    }
    for component in [Component::Server, Component::Web, Component::Themes] {
        if !config.component_roots.contains_key(&component)
            || !config.targets.contains_key(&component)
            || !config.bootstrap_versions.contains_key(&component)
        {
            return Err(UpdateError::Security(
                "root config is missing a Linux component".into(),
            ));
        }
    }
    for path in [&config.public_key, &config.server_config] {
        validate_root_owned_file(path, 1024 * 1024)?;
    }
    if !recovery_only {
        validate_root_owned_file(&config.health_ca_certificate, 1024 * 1024)?;
    }
    for path in [&config.state_root, &config.backup_root] {
        if !path.is_absolute() {
            return Err(UpdateError::Security(
                "configured roots must be absolute".into(),
            ));
        }
    }
    validate_root_owned_directory(&config.trust_root)?;
    validate_root_owned_directory(&config.cache_root)?;
    for root in config.component_roots.values() {
        validate_root_owned_directory(root)?;
    }
    if !config.maintenance_flag.is_absolute() || !config.quiescence_status.is_absolute() {
        return Err(UpdateError::Security(
            "runtime paths must be absolute".into(),
        ));
    }
    let url = reqwest::Url::parse(&config.health_url)
        .map_err(|_| UpdateError::Invalid("invalid health URL".into()))?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(UpdateError::Security(
            "health URL must be bounded HTTPS without credentials".into(),
        ));
    }
    if let Some(resolve) = &config.health_resolve {
        let address: SocketAddr = resolve
            .parse()
            .map_err(|_| UpdateError::Invalid("invalid fixed health resolution".into()))?;
        if !private_health_ip(address.ip()) || url.port_or_known_default() != Some(address.port()) {
            return Err(UpdateError::Security(
                "health resolution must be private and match the URL port".into(),
            ));
        }
    } else if !matches!(
        url.host_str(),
        Some("127.0.0.1" | "localhost" | "[::1]" | "::1")
    ) {
        return Err(UpdateError::Security(
            "non-loopback health URL requires fixed private resolution".into(),
        ));
    }
    let status_url = reqwest::Url::parse(&config.status_url)
        .map_err(|_| UpdateError::Invalid("invalid status URL".into()))?;
    if status_url.scheme() != url.scheme()
        || status_url.host_str() != url.host_str()
        || status_url.port_or_known_default() != url.port_or_known_default()
        || status_url.path() != "/api/2026-10-14/admin/updates"
        || status_url.query().is_some()
        || status_url.fragment().is_some()
        || !status_url.username().is_empty()
        || status_url.password().is_some()
    {
        return Err(UpdateError::Security(
            "status URL must be the fixed same-origin update endpoint".into(),
        ));
    }
    Ok(config)
}

fn inspect(config: &RootConfig) -> chill_updates::Result<()> {
    let state = state_store(config)?.read()?;
    let trust = AntiRollbackStore::open_root(&config.trust_root)?;
    let mut sequences = BTreeMap::new();
    let mut installed = BTreeMap::new();
    for component in [Component::Server, Component::Web, Component::Themes] {
        sequences.insert(component, trust.highest(component)?);
        installed.insert(component, installed_version(config, component)?.to_string());
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "repository": format!("{FIXED_GITHUB_OWNER}/{FIXED_GITHUB_REPOSITORY}"),
            "installed": installed, "sequences": sequences, "state": state
        }))?
    );
    Ok(())
}

enum CheckResult {
    Available,
    NoUpdate,
}

fn perform_check(config: &RootConfig, component: Component) -> chill_updates::Result<CheckResult> {
    require_linux_component(component)?;
    let now = OffsetDateTime::now_utc();
    let store = state_store(config)?;
    let operation = store.admit_intent(component, IntentKind::Check, now)?;
    if operation.status != OperationStatus::Pending {
        println!("{}", operation.id);
        return match (operation.status, operation.result_code.as_deref()) {
            (OperationStatus::Succeeded, Some("available")) => Ok(CheckResult::Available),
            (OperationStatus::Succeeded, Some("no_update")) => Ok(CheckResult::NoUpdate),
            _ => Err(UpdateError::Busy(
                "cached check is not a successful availability result".into(),
            )),
        };
    }
    store.transition(
        operation.id,
        OperationStatus::Pending,
        OperationStatus::Running,
        None,
        now,
    )?;
    let outcome = (|| {
        if AntiRollbackStore::open_root(&config.trust_root)?
            .pending(component)?
            .is_some()
        {
            // Never evict the only exact recovery bytes when a newer release
            // appears. Local root must finish the accepted transaction first.
            return Err(UpdateError::Security("accepted release requires local --retry-accepted recovery before checking another release".into()));
        }
        let client = GithubReleaseClient::new(TransportLimits::default())?;
        let candidate = client.find_latest(component)?.ok_or_else(|| {
            UpdateError::Invalid("no stable component release with signed manifest".into())
        })?;
        let installed = installed_version(config, component)?;
        if candidate.version <= installed {
            return Ok(None);
        }
        let (manifest_bytes, signature) = client.download_manifest(&candidate)?;
        let public_key = String::from_utf8(read_bounded(&config.public_key, 16 * 1024)?)
            .map_err(|_| UpdateError::Security("public key is not UTF-8".into()))?;
        let target = config
            .targets
            .get(&component)
            .ok_or_else(|| UpdateError::Security("component target missing".into()))?;
        let highest = AntiRollbackStore::open_root(&config.trust_root)?.highest(component)?;
        let context = verification_context(config, component, target, &installed, highest, now);
        let verified =
            parse_and_verify_manifest(&manifest_bytes, &signature, &public_key, &context)?;
        if candidate.tag != verified.manifest.release_tag
            || candidate.version != verified.manifest.version
        {
            return Err(UpdateError::Security(
                "GitHub release identity differs from signed manifest".into(),
            ));
        }
        let cache_key = hex::encode(Sha256::digest(&manifest_bytes));
        cleanup_component_cache(config, component, Some(&cache_key))?;
        ensure_cache_headroom(config, verified.selected_asset.byte_length)?;
        let root = config.cache_root.join(component.as_str());
        publish_cached_release(
            &root,
            &cache_key,
            &manifest_bytes,
            &signature,
            verified.selected_asset.byte_length,
            &verified.selected_asset.sha256,
            |payload| {
                client
                    .download_verified(
                        &verified.selected_asset.url,
                        payload,
                        verified.selected_asset.byte_length,
                        &verified.selected_asset.sha256,
                    )
                    .map(|_| ())
            },
        )?;
        write_available(
            config,
            &AvailableReceipt {
                schema_version: 1,
                operation_id: operation.id,
                component,
                target: target.clone(),
                release_tag: verified.manifest.release_tag,
                cache_key,
            },
        )?;
        publish_availability(
            config,
            component,
            Some(AvailabilityEntry {
                version: verified.manifest.version,
                source_commit: verified.manifest.source_commit,
                release_sequence: verified.manifest.release_sequence,
                checked_unix: now.unix_timestamp(),
            }),
        )?;
        Ok(Some(()))
    })();
    match outcome {
        Ok(Some(())) => {
            store.transition(
                operation.id,
                OperationStatus::Running,
                OperationStatus::Succeeded,
                Some("available"),
                OffsetDateTime::now_utc(),
            )?;
            println!("{}", operation.id);
            Ok(CheckResult::Available)
        }
        Ok(None) => {
            publish_availability(config, component, None)?;
            store.transition(
                operation.id,
                OperationStatus::Running,
                OperationStatus::Succeeded,
                Some("no_update"),
                OffsetDateTime::now_utc(),
            )?;
            println!("{} no-update", operation.id);
            Ok(CheckResult::NoUpdate)
        }
        Err(error) => {
            let _ = store.transition(
                operation.id,
                OperationStatus::Running,
                OperationStatus::Failed,
                Some("check_failed"),
                OffsetDateTime::now_utc(),
            );
            Err(error)
        }
    }
}

/// Publish only complete, verified bytes at the content-addressed cache path.
/// The downloader is a private seam: production always supplies the fixed
/// GitHub client, while tests can interrupt its write without network access.
fn publish_cached_release(
    root: &Path,
    cache_key: &str,
    manifest: &[u8],
    signature: &str,
    length: u64,
    digest: &str,
    download: impl FnOnce(&Path) -> chill_updates::Result<()>,
) -> chill_updates::Result<()> {
    let directory = root.join(cache_key);
    if !directory.exists() {
        let stage = root.join(format!(".download-{}", Uuid::new_v4()));
        fs::create_dir(&stage)?;
        write_new_sync(&stage.join("manifest.json"), manifest)?;
        write_new_sync(&stage.join("manifest.json.minisig"), signature.as_bytes())?;
        download(&stage.join("payload"))?;
        verify_staged_artifact(&stage.join("payload"), length, digest)?;
        File::open(&stage)?.sync_all()?;
        match fs::rename(&stage, &directory) {
            Ok(()) => File::open(root)?.sync_all()?,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                validate_cache_tree(&directory, 1024 * 1024 * 1024)?;
                fs::remove_dir_all(&stage)?;
            }
            Err(error) => return Err(error.into()),
        }
    } else {
        verify_staged_artifact(&directory.join("payload"), length, digest)?;
    }
    Ok(())
}

struct CachedVerification {
    verified: chill_updates::VerifiedManifest,
    manifest_sha256: String,
    accepted_retry: bool,
}

struct RootAcceptance {
    component: Component,
    sequence: u64,
    digest: String,
    version: Version,
    issued: OffsetDateTime,
    expires: OffsetDateTime,
    automatic: bool,
}

fn verify_cached(
    config: &RootConfig,
    component: Component,
) -> chill_updates::Result<CachedVerification> {
    let receipt = read_available(config, component)?;
    let directory = config
        .cache_root
        .join(component.as_str())
        .join(&receipt.cache_key);
    let manifest = read_bounded(&directory.join("manifest.json"), 1024 * 1024)?;
    let signature = String::from_utf8(read_bounded(
        &directory.join("manifest.json.minisig"),
        16 * 1024,
    )?)
    .map_err(|_| UpdateError::Security("signature is not UTF-8".into()))?;
    let public_key = String::from_utf8(read_bounded(&config.public_key, 16 * 1024)?)
        .map_err(|_| UpdateError::Security("public key is not UTF-8".into()))?;
    let manifest_sha256 = hex::encode(Sha256::digest(&manifest));
    let trust = AntiRollbackStore::open_root(&config.trust_root)?;
    let (highest, pending) = trust.verification_floor(component, &manifest_sha256)?;
    let actual_installed = installed_version(config, component)?;
    let verification_installed = if pending.is_some() {
        Version::new(0, 0, 0)
    } else {
        actual_installed
    };
    let verification_time = if let Some(pending) = &pending {
        let accepted = OffsetDateTime::from_unix_timestamp(pending.accepted_unix)
            .map_err(|_| UpdateError::Security("invalid root acceptance time".into()))?;
        if accepted > OffsetDateTime::now_utc() {
            return Err(UpdateError::Security(
                "root acceptance is in the future".into(),
            ));
        }
        accepted
    } else {
        OffsetDateTime::now_utc()
    };
    let context = verification_context(
        config,
        component,
        &receipt.target,
        &verification_installed,
        highest,
        verification_time,
    );
    let verified = parse_and_verify_manifest(&manifest, &signature, &public_key, &context)?;
    if verified.manifest.release_tag != receipt.release_tag {
        return Err(UpdateError::Security(
            "available receipt does not match manifest".into(),
        ));
    }
    if let Some(pending) = &pending
        && (pending.sequence != verified.manifest.release_sequence
            || pending.version != verified.manifest.version)
    {
        return Err(UpdateError::Security(
            "accepted manifest identity changed during retry".into(),
        ));
    }
    Ok(CachedVerification {
        verified,
        manifest_sha256,
        accepted_retry: pending.is_some(),
    })
}

fn perform_apply(
    config: &RootConfig,
    component: Component,
    first_install: bool,
    force: bool,
    retry_accepted: bool,
    automatic: bool,
) -> chill_updates::Result<()> {
    require_linux_component(component)?;
    let store = state_store(config)?;
    let preflight = (|| {
        Ok::<_, UpdateError>((
            read_available(config, component)?,
            verify_cached(config, component)?,
        ))
    })();
    let (receipt, cached) = match preflight {
        Ok(value) => value,
        Err(error) => {
            fail_component_intents(&store, component, "recheck_required")?;
            return Err(error);
        }
    };
    // The service-owned operation ledger cannot authorize retrying a release
    // already accepted by root and then failed/interrupted. Only local root can.
    if cached.accepted_retry && !retry_accepted {
        fail_component_intents(&store, component, "local_recovery_required")?;
        return Err(UpdateError::Security(
            "accepted release requires local --retry-accepted recovery".into(),
        ));
    }
    if cached.accepted_retry && retry_accepted {
        rescue(config)?;
    }
    reconcile_store(config, &store)?;
    let verified = cached.verified;
    let operation = store.admit_install(
        component,
        &cached.manifest_sha256,
        automatic,
        OffsetDateTime::now_utc(),
    )?;
    if !matches!(
        operation.status,
        OperationStatus::Pending | OperationStatus::DeferredBusy
    ) {
        return Err(UpdateError::Busy(format!(
            "install operation {} is already {:?}",
            operation.id, operation.status
        )));
    }
    if operation.automatic
        && store
            .read()?
            .policies
            .get(&component)
            .is_some_and(|policy| policy.mode != PolicyMode::Automatic)
    {
        store.transition(
            operation.id,
            operation.status,
            OperationStatus::Failed,
            Some("automatic_policy_changed"),
            OffsetDateTime::now_utc(),
        )?;
        return Err(UpdateError::Busy(
            "automatic installation was disabled".into(),
        ));
    }
    store.transition(
        operation.id,
        operation.status,
        OperationStatus::Running,
        None,
        OffsetDateTime::now_utc(),
    )?;
    let _running = RunningOperation {
        store: &store,
        id: operation.id,
    };
    let component_root = config
        .component_roots
        .get(&component)
        .ok_or_else(|| UpdateError::Security("component root missing".into()))?
        .clone();
    let layout = InstallLayout {
        component_root,
        backup_root: config.backup_root.clone(),
    };
    let artifact = config
        .cache_root
        .join(component.as_str())
        .join(&receipt.cache_key)
        .join("payload");
    verify_staged_artifact(
        &artifact,
        verified.selected_asset.byte_length,
        &verified.selected_asset.sha256,
    )?;
    let mut hooks = LinuxHooks::new(config)?;
    hooks.acceptance = Some(RootAcceptance {
        component,
        sequence: verified.manifest.release_sequence,
        digest: cached.manifest_sha256.clone(),
        version: verified.manifest.version.clone(),
        automatic: operation.automatic,
        issued: OffsetDateTime::parse(
            &verified.manifest.issued_at,
            &time::format_description::well_known::Rfc3339,
        )
        .map_err(|_| UpdateError::Security("invalid issuance time".into()))?,
        expires: OffsetDateTime::parse(
            &verified.manifest.expires_at,
            &time::format_description::well_known::Rfc3339,
        )
        .map_err(|_| UpdateError::Security("invalid expiry time".into()))?,
    });
    let expected_target = PathBuf::from("releases").join(format!(
        "{}-{}",
        verified.manifest.version, verified.manifest.release_sequence
    ));
    let resumes_healthy_candidate = cached.accepted_retry
        && !layout.component_root.join("transaction.json").exists()
        && fs::read_link(layout.component_root.join("current"))
            .ok()
            .as_ref()
            == Some(&expected_target);
    if !resumes_healthy_candidate && !layout.component_root.join("transaction.json").exists() {
        let orphan = layout.component_root.join(&expected_target);
        if let Ok(metadata) = fs::symlink_metadata(&orphan) {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(UpdateError::Security(
                    "accepted retry candidate is not a directory".into(),
                ));
            }
            fs::remove_dir_all(orphan)?;
            File::open(&layout.component_root)?.sync_all()?;
        }
    }
    let releases = layout.component_root.join("releases");
    if releases.exists() {
        for entry in fs::read_dir(&releases)? {
            let entry = entry?;
            if entry
                .file_name()
                .to_str()
                .and_then(|name| name.strip_prefix(".stage-"))
                .is_some_and(|id| Uuid::parse_str(id).is_ok())
            {
                validate_cache_tree(&entry.path(), 1024 * 1024 * 1024)?;
                fs::remove_dir_all(entry.path())?;
            }
        }
        File::open(&releases)?.sync_all()?;
    }
    let trust = AntiRollbackStore::open_root(&config.trust_root)?;
    if resumes_healthy_candidate {
        let outcome = hooks.health_check(component, &layout.component_root.join(&expected_target));
        if let Err(error) = outcome {
            let _ = store.transition(
                operation.id,
                OperationStatus::Running,
                OperationStatus::Failed,
                Some("recovery_health_failed"),
                OffsetDateTime::now_utc(),
            );
            return Err(error);
        }
        store.transition(
            operation.id,
            OperationStatus::Running,
            OperationStatus::Succeeded,
            Some("recovered_after_health"),
            OffsetDateTime::now_utc(),
        )?;
        trust.commit_pending(
            component,
            verified.manifest.release_sequence,
            &cached.manifest_sha256,
        )?;
        if let Err(error) = publish_availability(config, component, None) {
            eprintln!("availability refresh failed: {error}");
        }
        println!("installed={} recovered", verified.manifest.version);
        return Ok(());
    }
    let request = ApplyRequest {
        operation_id: operation.id,
        verified: &verified,
        artifact: &artifact,
        layout: &layout,
        limits: &ExtractionLimits::default(),
        allow_first_install: first_install,
        force_interrupt: force,
    };
    let outcome = apply_staged(&request, &mut hooks);
    match outcome {
        Ok(result) => {
            store.transition(
                operation.id,
                OperationStatus::Running,
                OperationStatus::Succeeded,
                Some("installed"),
                OffsetDateTime::now_utc(),
            )?;
            trust.commit_pending(
                component,
                verified.manifest.release_sequence,
                &cached.manifest_sha256,
            )?;
            if let Err(error) = publish_availability(config, component, None) {
                eprintln!("availability refresh failed: {error}");
            }
            println!(
                "installed={} backup={}",
                result.installed.display(),
                result.backup_id
            );
            Ok(())
        }
        Err(error) => {
            // Busy admission happens before before_switch/acceptance. Once a
            // root sequence is accepted, never let a LAN intent reset recovery.
            let accepted = trust
                .verification_floor(component, &cached.manifest_sha256)?
                .1
                .is_some();
            let deferred = matches!(error, UpdateError::Busy(_)) && !accepted;
            let _ = store.transition(
                operation.id,
                OperationStatus::Running,
                if deferred {
                    OperationStatus::DeferredBusy
                } else {
                    OperationStatus::Failed
                },
                Some(if deferred {
                    if operation.automatic {
                        "deferred_busy_auto"
                    } else {
                        "deferred_busy_manual"
                    }
                } else {
                    "apply_failed"
                }),
                OffsetDateTime::now_utc(),
            );
            Err(error)
        }
    }
}

/// Ensure every admitted install has a terminal outcome even on a preflight
/// error. Drop never converts an already Deferred/Succeeded operation to Failed.
struct RunningOperation<'a> {
    store: &'a StateStore,
    id: Uuid,
}
impl Drop for RunningOperation<'_> {
    fn drop(&mut self) {
        let _ = self.store.transition(
            self.id,
            OperationStatus::Running,
            OperationStatus::Failed,
            Some("interrupted_preflight"),
            OffsetDateTime::now_utc(),
        );
    }
}

fn fail_component_intents(
    store: &StateStore,
    component: Component,
    code: &str,
) -> chill_updates::Result<()> {
    store.update_scheduler(|state| {
        for operation in &mut state.operations {
            if operation.component == component
                && operation.kind == IntentKind::Install
                && matches!(
                    operation.status,
                    OperationStatus::Pending
                        | OperationStatus::Running
                        | OperationStatus::DeferredBusy
                )
            {
                operation.status = OperationStatus::Failed;
                operation.result_code = Some(code.to_owned());
                operation.updated_unix = OffsetDateTime::now_utc().unix_timestamp();
            }
        }
        Ok(())
    })
}

fn reconcile_store(config: &RootConfig, store: &StateStore) -> chill_updates::Result<()> {
    // Reconcile under the helper lease, before coalescing new intents. Only
    // pre-acceptance crashes are automatically retryable; root-pending slots
    // require explicit local recovery, never a service-writable status field.
    let trust = AntiRollbackStore::open_root(&config.trust_root)?;
    let recovery_required = [Component::Server, Component::Web, Component::Themes]
        .into_iter()
        .map(|component| {
            trust
                .pending(component)
                .map(|pending| (component, pending.is_some()))
        })
        .collect::<chill_updates::Result<BTreeMap<_, _>>>()?;
    store.update_scheduler(|state| {
        let now = OffsetDateTime::now_utc().unix_timestamp();
        for operation in &mut state.operations {
            if matches!(
                operation.status,
                OperationStatus::Pending | OperationStatus::Running | OperationStatus::DeferredBusy
            ) && ((operation.automatic
                && state
                    .policies
                    .get(&operation.component)
                    .is_some_and(|policy| policy.mode != PolicyMode::Automatic))
                || operation.attempts >= 3)
            {
                operation.status = OperationStatus::Failed;
                operation.result_code = Some(
                    if operation.attempts >= 3 {
                        "retry_exhausted"
                    } else {
                        "automatic_policy_changed"
                    }
                    .into(),
                );
                operation.updated_unix = now;
                continue;
            }
            if operation.kind == IntentKind::Install
                && recovery_required
                    .get(&operation.component)
                    .copied()
                    .unwrap_or(true)
                && matches!(
                    operation.status,
                    OperationStatus::Pending
                        | OperationStatus::Running
                        | OperationStatus::DeferredBusy
                )
            {
                operation.status = OperationStatus::Failed;
                operation.result_code = Some("local_recovery_required".into());
                operation.updated_unix = now;
                continue;
            }
            if operation.status == OperationStatus::Running {
                operation.status = if operation.kind == IntentKind::Install
                    && recovery_required
                        .get(&operation.component)
                        .copied()
                        .unwrap_or(true)
                {
                    OperationStatus::Failed
                } else {
                    OperationStatus::Pending
                };
                operation.result_code = Some(
                    if operation.status == OperationStatus::Failed {
                        "local_recovery_required"
                    } else {
                        "recovered_interrupted"
                    }
                    .into(),
                );
                operation.updated_unix = now;
            }
            if operation.kind == IntentKind::Install
                && operation.status == OperationStatus::DeferredBusy
                && operation.attempts >= 3
            {
                operation.status = OperationStatus::Failed;
                operation.result_code = Some("busy_retry_exhausted".into());
                operation.updated_unix = now;
            }
            if operation.status == OperationStatus::DeferredBusy
                && operation.result_code.as_deref() == Some("deferred_busy_auto")
                && state
                    .policies
                    .get(&operation.component)
                    .is_some_and(|policy| policy.mode != PolicyMode::Automatic)
            {
                operation.status = OperationStatus::Failed;
                operation.result_code = Some("automatic_policy_changed".into());
                operation.updated_unix = now;
            }
        }
        Ok(())
    })?;
    Ok(())
}

fn tick(config: &RootConfig) -> chill_updates::Result<()> {
    let offset = os_local_offset()?;
    let store = state_store(config)?;
    reconcile_store(config, &store)?;
    let pending = store
        .read()?
        .operations
        .into_iter()
        .filter(|operation| {
            matches!(
                operation.status,
                OperationStatus::Pending | OperationStatus::DeferredBusy
            ) && (operation.kind != IntentKind::Install
                || operation.status != OperationStatus::DeferredBusy
                || (operation.attempts < 3
                    && OffsetDateTime::now_utc()
                        .unix_timestamp()
                        .saturating_sub(operation.updated_unix)
                        >= 15 * 60))
        })
        .collect::<Vec<_>>();
    for operation in pending
        .iter()
        .filter(|operation| operation.kind == IntentKind::Check)
    {
        if let Err(error) = perform_check(config, operation.component) {
            eprintln!(
                "pending {} check failed: {error}",
                operation.component.as_str()
            );
        }
    }
    for operation in pending
        .iter()
        .filter(|operation| operation.kind == IntentKind::Install)
    {
        if let Err(error) = perform_apply(
            config,
            operation.component,
            false,
            false,
            false,
            operation.automatic,
        ) {
            eprintln!(
                "pending {} install failed: {error}",
                operation.component.as_str()
            );
        }
    }
    let attempts = store.update_scheduler(|state| {
        due_attempts_for(
            state,
            OffsetDateTime::now_utc(),
            offset,
            &[Component::Server, Component::Web, Component::Themes],
        )
    })?;
    let policies = store.read()?.policies;
    for attempt in attempts {
        let check = perform_check(config, attempt.component);
        if matches!(check, Ok(CheckResult::Available))
            && policies
                .get(&attempt.component)
                .is_some_and(|policy| policy.mode == PolicyMode::Automatic)
        {
            perform_apply(config, attempt.component, false, false, false, true)?;
        } else if let Err(error) = check {
            eprintln!(
                "scheduled {} check deferred: {error}",
                attempt.component.as_str()
            );
            let _ = store.update_scheduler(|state| {
                record_retry(state, attempt.component, OffsetDateTime::now_utc())
            });
        }
    }
    Ok(())
}

fn verification_context<'a>(
    config: &'a RootConfig,
    component: Component,
    target: &'a str,
    installed: &'a Version,
    highest: u64,
    now: OffsetDateTime,
) -> VerificationContext<'a> {
    VerificationContext {
        now,
        expected_key_id: &config.key_id,
        component,
        target,
        installed_version: installed,
        highest_sequence: highest,
        runtime: RuntimeCompatibility {
            api_version: Some(config.api_version.clone()),
            server_version: installed_version(config, Component::Server).ok(),
            database_schema: Some(config.database_schema),
            renderer_version: Some(config.renderer_version),
            theme_format: Some(config.theme_format),
        },
    }
}

fn state_store(config: &RootConfig) -> chill_updates::Result<StateStore> {
    StateStore::open_for_owner(&config.state_root, config.service_uid, config.service_gid)
}

fn installed_version(config: &RootConfig, component: Component) -> chill_updates::Result<Version> {
    let root = config
        .component_roots
        .get(&component)
        .ok_or_else(|| UpdateError::Security("component root missing".into()))?;
    match fs::read_link(root.join("current")) {
        Ok(target) => {
            let leaf = target
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or_else(|| UpdateError::Security("current slot target is invalid".into()))?;
            let version = leaf
                .rsplit_once('-')
                .map(|(version, _)| version)
                .ok_or_else(|| UpdateError::Security("current slot has no sequence".into()))?;
            Version::parse(version)
                .map_err(|_| UpdateError::Security("current slot version is invalid".into()))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => config
            .bootstrap_versions
            .get(&component)
            .cloned()
            .ok_or_else(|| UpdateError::Security("bootstrap version missing".into())),
        Err(error) => Err(error.into()),
    }
}

fn parse_component(value: &str) -> chill_updates::Result<Component> {
    Component::from_str(value)
}
fn require_linux_component(component: Component) -> chill_updates::Result<()> {
    if component == Component::Mac {
        Err(UpdateError::Invalid(
            "Mac updates use mandatory-confirmation Tauri integration".into(),
        ))
    } else {
        Ok(())
    }
}

/// Fixed root-only failure path used by the updater unit's ExecStopPost. It
/// accepts no component/path/command input, repairs every component marker,
/// then asks systemd to restore the known service unit.
fn rescue(config: &RootConfig) -> chill_updates::Result<()> {
    let interrupted = config
        .component_roots
        .values()
        .any(|root| root.join("transaction.json").exists());
    if interrupted {
        // A candidate can already be running when its parent helper is killed.
        // Stop it before deleting/rolling back its slot; `start` alone is a no-op
        // for an active, uncommitted executable.
        stop_service_for_recovery(&config.service_name)?;
    }
    for component in [Component::Server, Component::Web, Component::Themes] {
        let root = config
            .component_roots
            .get(&component)
            .ok_or_else(|| UpdateError::Security("component root missing".into()))?;
        recover_interrupted(root, Some(&activation_authorization(config, component)))?;
    }
    match fs::remove_file(&config.maintenance_flag) {
        Ok(()) => File::open(config.maintenance_flag.parent().unwrap())?.sync_all()?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
        Err(error) => return Err(error.into()),
    }
    let mut command = Command::new("/usr/bin/systemctl");
    command.args(["start", config.service_name.as_str()]);
    run_bounded_backup(&mut command, Duration::from_secs(30))
        .map_err(|_| UpdateError::Invalid("rescue could not start service".into()))?;
    Ok(())
}

/// PID 1 owns the service's 30-second graceful-stop deadline. The supervising
/// command must outlive that deadline: killing `systemctl` at the same instant
/// used to abandon recovery just before PID 1 killed an unresponsive candidate.
fn stop_service_for_recovery(service: &str) -> chill_updates::Result<()> {
    let mut stop = Command::new("/usr/bin/systemctl");
    stop.args(["stop", service]);
    if run_bounded_backup(&mut stop, Duration::from_secs(45)).is_err() {
        // A failed/slow D-Bus job must not leave the uncommitted executable
        // running while its slot is removed. The unit uses KillMode=control-group.
        let mut kill = Command::new("/usr/bin/systemctl");
        kill.args(["kill", "--kill-whom=all", "--signal=KILL", service]);
        let _ = run_bounded_backup(&mut kill, Duration::from_secs(10));
        let mut stop = Command::new("/usr/bin/systemctl");
        stop.args(["stop", "--no-block", service]);
        run_bounded_backup(&mut stop, Duration::from_secs(10))?;
    }
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let mut show = Command::new("/usr/bin/systemctl");
        show.args([
            "show",
            service,
            "-p",
            "ActiveState",
            "-p",
            "MainPID",
            "-p",
            "ControlPID",
        ]);
        let output = run_bounded_backup(&mut show, Duration::from_secs(5))?;
        let output = String::from_utf8_lossy(&output);
        let values: BTreeMap<_, _> = output
            .lines()
            .filter_map(|line| line.split_once('='))
            .collect();
        if matches!(values.get("ActiveState"), Some(&"inactive" | &"failed"))
            && values.get("MainPID") == Some(&"0")
            && values.get("ControlPID") == Some(&"0")
        {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(UpdateError::Busy(
                "recovery could not confirm service stopped".into(),
            ));
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn available_path(config: &RootConfig, component: Component) -> PathBuf {
    config
        .cache_root
        .join(component.as_str())
        .join("available.json")
}

fn cleanup_component_cache(
    config: &RootConfig,
    component: Component,
    keep: Option<&str>,
) -> chill_updates::Result<()> {
    let root = config.cache_root.join(component.as_str());
    match fs::create_dir(&root) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error.into()),
    }
    validate_root_owned_directory(&root)?;
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name
            .to_str()
            .ok_or_else(|| UpdateError::Security("non-UTF8 cache entry refused".into()))?;
        if name == "available.json" || keep == Some(name) {
            continue;
        }
        if let Some(temporary_kind) = abandoned_cache_temporary_kind(name) {
            let entry_kind = entry.file_type()?;
            if matches!(temporary_kind, CacheTemporary::Download) && entry_kind.is_dir() {
                validate_cache_tree(&entry.path(), 1024 * 1024 * 1024)?;
                fs::remove_dir_all(entry.path())?;
            } else if matches!(temporary_kind, CacheTemporary::Available) && entry_kind.is_file() {
                if entry.metadata()?.len() > 64 * 1024 {
                    return Err(UpdateError::Security(
                        "temporary receipt exceeds bound".into(),
                    ));
                }
                fs::remove_file(entry.path())?;
            } else {
                return Err(UpdateError::Security(
                    "cache temporary is not regular".into(),
                ));
            }
            continue;
        }
        if name.len() != 64 || !name.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(UpdateError::Security(
                "unexpected cache entry refused".into(),
            ));
        }
        validate_cache_tree(&entry.path(), 1024 * 1024 * 1024)?;
        fs::remove_dir_all(entry.path())?;
    }
    File::open(&root)?.sync_all()?;
    Ok(())
}

#[derive(Clone, Copy)]
enum CacheTemporary {
    Download,
    Available,
}
fn abandoned_cache_temporary_kind(name: &str) -> Option<CacheTemporary> {
    if let Some(id) = name.strip_prefix(".download-") {
        return Uuid::parse_str(id).ok().map(|_| CacheTemporary::Download);
    }
    name.strip_prefix(".available-")?
        .strip_suffix(".tmp")
        .and_then(|id| Uuid::parse_str(id).ok())
        .map(|_| CacheTemporary::Available)
}

fn ensure_cache_headroom(config: &RootConfig, incoming: u64) -> chill_updates::Result<()> {
    const GLOBAL_CACHE_MAX: u64 = 3 * 1024 * 1024 * 1024;
    const HEADROOM: u64 = 128 * 1024 * 1024;
    let used = validate_cache_tree(&config.cache_root, GLOBAL_CACHE_MAX)?;
    if used.saturating_add(incoming) > GLOBAL_CACHE_MAX {
        return Err(UpdateError::Security(
            "global update cache quota exceeded".into(),
        ));
    }
    let mut stats: libc::statvfs = unsafe { std::mem::zeroed() };
    let path = std::ffi::CString::new(config.cache_root.as_os_str().as_encoded_bytes())
        .map_err(|_| UpdateError::Security("cache path contains NUL".into()))?;
    if unsafe { libc::statvfs(path.as_ptr(), &mut stats) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let available = (stats.f_bavail as u64).saturating_mul(stats.f_frsize as u64);
    if available < incoming.saturating_add(HEADROOM) {
        return Err(UpdateError::Busy(
            "insufficient disk headroom for verified download".into(),
        ));
    }
    Ok(())
}

fn validate_cache_tree(root: &Path, maximum: u64) -> chill_updates::Result<u64> {
    let mut total = 0u64;
    let mut entries = 0usize;
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            entries += 1;
            if entries > 4096 {
                return Err(UpdateError::Security("cache entry bound exceeded".into()));
            }
            let kind = entry.file_type()?;
            if kind.is_symlink() {
                return Err(UpdateError::Security("cache symlink refused".into()));
            }
            if kind.is_dir() {
                stack.push(entry.path());
            } else if kind.is_file() {
                total = total
                    .checked_add(entry.metadata()?.len())
                    .ok_or_else(|| UpdateError::Security("cache size overflow".into()))?;
                if total > maximum {
                    return Err(UpdateError::Security("cache byte bound exceeded".into()));
                }
            } else {
                return Err(UpdateError::Security("cache special file refused".into()));
            }
        }
    }
    Ok(total)
}

fn activation_authorization(config: &RootConfig, component: Component) -> PathBuf {
    config
        .maintenance_flag
        .parent()
        .unwrap_or(Path::new("/run/chillcenter-update"))
        .join(format!("activate-{}", component.as_str()))
}

fn validate_backup_receipt(
    receipt: &Path,
    backup_root: &Path,
    service_uid: u32,
    started_unix: i64,
) -> chill_updates::Result<String> {
    if !receipt.is_absolute() || receipt.parent() != Some(backup_root) {
        return Err(UpdateError::Security(
            "backup receipt escaped the configured private root".into(),
        ));
    }
    let identifier = receipt
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| UpdateError::Security("backup receipt identifier is invalid".into()))?;
    Uuid::parse_str(identifier)
        .map_err(|_| UpdateError::Security("backup receipt is not a UUID".into()))?;
    let metadata = fs::symlink_metadata(receipt)?;
    if metadata.file_type().is_symlink()
        || !metadata.is_dir()
        || metadata.uid() != service_uid
        || metadata.mode() & 0o077 != 0
        || metadata.mtime() < started_unix.saturating_sub(1)
    {
        return Err(UpdateError::Security(
            "backup receipt directory owner, mode, or freshness is invalid".into(),
        ));
    }
    let manifest: BackupManifest = parse_strict_json(&read_bounded(
        &receipt.join("manifest.json"),
        4 * 1024 * 1024,
    )?)?;
    if manifest.format_version != 1
        || manifest.schema_version != 3
        || manifest.data_root != backup_root.parent().unwrap_or(Path::new("/invalid"))
        || manifest.entries.is_empty()
        || manifest.entries.len() > 20_008
    {
        return Err(UpdateError::Security(
            "backup manifest identity is invalid".into(),
        ));
    }
    let mut expected = std::collections::BTreeSet::new();
    let mut total = 0u64;
    for entry in &manifest.entries {
        if entry.path.contains('\\')
            || entry.sha256.len() != 64
            || !entry.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
            || Path::new(&entry.path)
                .components()
                .any(|part| !matches!(part, std::path::Component::Normal(_)))
            || !expected.insert(entry.path.clone())
        {
            return Err(UpdateError::Security("backup entry is invalid".into()));
        }
        total = total
            .checked_add(entry.size)
            .ok_or_else(|| UpdateError::Security("backup byte total overflow".into()))?;
        if total > 1024 * 1024 * 1024 {
            return Err(UpdateError::Security("backup exceeds byte bound".into()));
        }
        let path = receipt.join(&entry.path);
        let file_metadata = fs::symlink_metadata(&path)?;
        if file_metadata.file_type().is_symlink()
            || !file_metadata.is_file()
            || file_metadata.uid() != service_uid
            || file_metadata.len() != entry.size
            || hash_file(&path)? != entry.sha256
        {
            return Err(UpdateError::Security(
                "backup entry hash/metadata mismatch".into(),
            ));
        }
    }
    for required in [
        "catalog.sqlite",
        "server-config.json",
        "secrets/tls/certificate.pem",
        "secrets/tls/service-key.pem",
    ] {
        if !expected.contains(required) {
            return Err(UpdateError::Security(
                "backup omits database, config, or TLS material".into(),
            ));
        }
    }
    let mut actual = std::collections::BTreeSet::new();
    let mut stack = vec![receipt.to_path_buf()];
    while let Some(directory) = stack.pop() {
        for item in fs::read_dir(&directory)? {
            let item = item?;
            let kind = item.file_type()?;
            if kind.is_symlink() {
                return Err(UpdateError::Security("backup contains a symlink".into()));
            }
            if kind.is_dir() {
                stack.push(item.path());
            } else if kind.is_file() {
                let relative = item
                    .path()
                    .strip_prefix(receipt)
                    .map_err(|_| UpdateError::Security("backup path escaped".into()))?
                    .to_string_lossy()
                    .into_owned();
                actual.insert(relative);
            } else {
                return Err(UpdateError::Security(
                    "backup contains a special file".into(),
                ));
            }
            if actual.len() + stack.len() > 20_009 {
                return Err(UpdateError::Security("backup entry count exceeded".into()));
            }
        }
    }
    expected.insert("manifest.json".into());
    if actual != expected {
        return Err(UpdateError::Security(
            "backup actual files differ from signed receipt".into(),
        ));
    }
    File::open(receipt)?.sync_all()?;
    File::open(backup_root)?.sync_all()?;
    Ok(identifier.to_owned())
}

fn hash_file(path: &Path) -> chill_updates::Result<String> {
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hex::encode(hasher.finalize()))
}
fn write_available(config: &RootConfig, receipt: &AvailableReceipt) -> chill_updates::Result<()> {
    let path = available_path(config, receipt.component);
    let parent = path.parent().unwrap().to_path_buf();
    fs::create_dir_all(&parent)?;
    let temporary = parent.join(format!(".available-{}.tmp", receipt.operation_id));
    write_new_sync(&temporary, &serde_json::to_vec_pretty(receipt)?)?;
    fs::rename(temporary, &path)?;
    File::open(&parent)?.sync_all()?;
    Ok(())
}
fn read_available(
    config: &RootConfig,
    component: Component,
) -> chill_updates::Result<AvailableReceipt> {
    let receipt: AvailableReceipt = parse_strict_json(&read_bounded(
        &available_path(config, component),
        64 * 1024,
    )?)?;
    if receipt.schema_version != 1
        || receipt.component != component
        || receipt.target.len() > 128
        || receipt.release_tag.len() > 128
        || receipt.cache_key.len() != 64
        || !receipt
            .cache_key
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(UpdateError::Security("available receipt is invalid".into()));
    }
    Ok(receipt)
}

fn publish_availability(
    config: &RootConfig,
    component: Component,
    entry: Option<AvailabilityEntry>,
) -> chill_updates::Result<()> {
    // UI-only snapshot. It is never consulted by signature, sequence, path, or
    // install decisions; those are revalidated from the root-owned cache/trust store.
    let (state_handle, pinned_root) = pin_service_state_root(config)?;
    let path = pinned_root.join("available.json");
    let mut entries: BTreeMap<Component, AvailabilityEntry> = match read_bounded(&path, 64 * 1024) {
        Ok(bytes) => parse_strict_json(&bytes)?,
        Err(UpdateError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            BTreeMap::new()
        }
        Err(error) => return Err(error),
    };
    if let Some(entry) = entry {
        entries.insert(component, entry);
    } else {
        entries.remove(&component);
    }
    if entries.len() > 4 {
        return Err(UpdateError::Security(
            "availability snapshot is unbounded".into(),
        ));
    }
    let bytes = serde_json::to_vec_pretty(&entries)?;
    let temporary = pinned_root.join(format!(".available-{}.tmp", Uuid::new_v4()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o644)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(&temporary)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    if unsafe { libc::fchown(file.as_raw_fd(), 0, config.service_gid) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    fs::rename(&temporary, &path)?;
    state_handle.sync_all()?;
    Ok(())
}

fn pin_service_state_root(config: &RootConfig) -> chill_updates::Result<(File, PathBuf)> {
    let handle = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_DIRECTORY)
        .open(&config.state_root)?;
    let metadata = handle.metadata()?;
    if !metadata.is_dir()
        || metadata.uid() != config.service_uid
        || metadata.gid() != config.service_gid
        || metadata.mode() & 0o022 != 0
    {
        return Err(UpdateError::Security(
            "service state root owner or mode is invalid".into(),
        ));
    }
    let pinned = PathBuf::from(format!("/proc/self/fd/{}", handle.as_raw_fd()));
    Ok((handle, pinned))
}

fn read_bounded(path: &Path, maximum: u64) -> chill_updates::Result<Vec<u8>> {
    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    let file = options.open(path)?;
    if file.metadata()?.len() > maximum {
        return Err(UpdateError::Security("file exceeds bound".into()));
    }
    let mut bytes = Vec::new();
    file.take(maximum + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > maximum {
        return Err(UpdateError::Security("file exceeds bound".into()));
    }
    Ok(bytes)
}
fn write_new_sync(path: &Path, bytes: &[u8]) -> chill_updates::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

const BACKUP_OUTPUT_LIMIT: usize = 64 * 1024;

/// Run a service-owned backup without allowing it to wedge the root updater or
/// retain unbounded output. The child is a process-group leader so a timeout or
/// pipe overflow terminates descendants as well as the direct child.
fn run_bounded_backup(command: &mut Command, deadline: Duration) -> chill_updates::Result<Vec<u8>> {
    use std::sync::mpsc;
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    unsafe {
        command.pre_exec(|| {
            if libc::setpgid(0, 0) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = command.spawn()?;
    let pid = child.id() as i32;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| UpdateError::Invalid("backup stdout missing".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| UpdateError::Invalid("backup stderr missing".into()))?;
    let (sender, receiver) = mpsc::channel();
    let stdout_sender = sender.clone();
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = stdout
            .take((BACKUP_OUTPUT_LIMIT / 2 + 1) as u64)
            .read_to_end(&mut bytes)
            .map(|_| bytes);
        let _ = stdout_sender.send((true, result));
    });
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = stderr
            .take((BACKUP_OUTPUT_LIMIT / 2 + 1) as u64)
            .read_to_end(&mut bytes)
            .map(|_| bytes);
        let _ = sender.send((false, result));
    });
    let started = Instant::now();
    let mut stdout_result = None;
    let mut overflow = false;
    let mut completed = 0;
    let mut status;
    loop {
        status = child.try_wait()?;
        if overflow || started.elapsed() >= deadline || (completed == 2 && status.is_some()) {
            break;
        }
        if completed == 2 {
            // Closing both pipes precedes the kernel's exit/reap notification.
            // It is not a timeout and must not kill a successful command.
            thread::sleep(Duration::from_millis(2));
            continue;
        }
        match receiver.recv_timeout(Duration::from_millis(20)) {
            Ok((is_stdout, Ok(bytes))) => {
                overflow |= bytes.len() > BACKUP_OUTPUT_LIMIT / 2;
                if is_stdout {
                    stdout_result = Some(bytes);
                }
                completed += 1;
            }
            Ok((_, Err(_))) => {
                overflow = true;
                break;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                overflow = completed != 2;
            }
        }
    }
    if status.is_none() || overflow || completed != 2 {
        unsafe {
            libc::kill(-pid, libc::SIGKILL);
        }
        let _ = child.wait();
        while receiver.recv_timeout(Duration::from_millis(50)).is_ok() {}
        return Err(UpdateError::Busy(
            "offline server backup exceeded time or output bound".into(),
        ));
    }
    let status = status.unwrap();
    if !status.success() {
        return Err(UpdateError::Invalid("offline server backup failed".into()));
    }
    // The pipes are read in creation order (stdout then stderr); only stdout
    // carries the receipt, while stderr remains bounded diagnostic discard.
    let stdout =
        stdout_result.ok_or_else(|| UpdateError::Invalid("backup stdout unavailable".into()))?;
    Ok(stdout)
}

struct LinuxHooks<'a> {
    config: &'a RootConfig,
    client: reqwest::blocking::Client,
    stopped: bool,
    maintenance_generation_ns: Option<String>,
    acceptance: Option<RootAcceptance>,
}

impl<'a> LinuxHooks<'a> {
    fn new(config: &'a RootConfig) -> chill_updates::Result<Self> {
        let certificate = reqwest::Certificate::from_pem(&read_bounded(
            &config.health_ca_certificate,
            1024 * 1024,
        )?)?;
        let mut builder = reqwest::blocking::Client::builder()
            .add_root_certificate(certificate)
            .redirect(RedirectPolicy::none())
            .timeout(Duration::from_secs(10));
        if let Some(resolve) = &config.health_resolve {
            let host = reqwest::Url::parse(&config.health_url)
                .map_err(|_| UpdateError::Invalid("invalid health URL".into()))?
                .host_str()
                .ok_or_else(|| UpdateError::Invalid("health URL has no host".into()))?
                .to_owned();
            let address: SocketAddr = resolve
                .parse()
                .map_err(|_| UpdateError::Invalid("invalid fixed health resolution".into()))?;
            builder = builder.resolve(&host, address);
        }
        let client = builder.build()?;
        Ok(Self {
            config,
            client,
            stopped: false,
            maintenance_generation_ns: None,
            acceptance: None,
        })
    }
    fn systemctl(&self, action: &str) -> chill_updates::Result<()> {
        if action == "stop" {
            return stop_service_for_recovery(&self.config.service_name);
        }
        let mut command = Command::new("/usr/bin/systemctl");
        command.args([action, self.config.service_name.as_str()]);
        run_bounded_backup(&mut command, Duration::from_secs(30))
            .map(|_| ())
            .map_err(|_| UpdateError::Invalid(format!("systemctl {action} failed")))
    }
    fn status(&self) -> chill_updates::Result<QuiescenceStatus> {
        let status: QuiescenceStatus =
            parse_strict_json(&read_bounded(&self.config.quiescence_status, 16 * 1024)?)?;
        let age = OffsetDateTime::now_utc()
            .unix_timestamp()
            .saturating_sub(status.updated_unix);
        if !(0..=3).contains(&age) {
            return Err(UpdateError::Busy("quiescence status is stale".into()));
        }
        if let Some(expected) = &self.maintenance_generation_ns
            && &status.maintenance_generation_ns != expected
        {
            return Err(UpdateError::Busy(
                "quiescence generation does not match maintenance flag".into(),
            ));
        }
        Ok(status)
    }
}

impl InstallHooks for LinuxHooks<'_> {
    fn is_idle(&mut self, _: Component) -> chill_updates::Result<bool> {
        let status = self.status()?;
        Ok(!status.busy && status.active_mutations == 0)
    }
    fn begin_quiescence(&mut self, component: Component, force: bool) -> chill_updates::Result<()> {
        if let Some(parent) = self.config.maintenance_flag.parent() {
            fs::create_dir_all(parent)?;
        }
        write_new_sync(
            &self.config.maintenance_flag,
            format!("component={}\nforce={}\n", component.as_str(), force).as_bytes(),
        )?;
        let modified = fs::metadata(&self.config.maintenance_flag)?
            .modified()?
            .duration_since(UNIX_EPOCH)
            .map_err(|_| UpdateError::Security("maintenance flag mtime predates epoch".into()))?;
        self.maintenance_generation_ns = Some(modified.as_nanos().to_string());
        let mut ready = false;
        for _ in 0..60 {
            if let Ok(status) = self.status()
                && status.quiescing
                && !status.busy
                && status.active_mutations == 0
            {
                ready = true;
                break;
            }
            thread::sleep(Duration::from_millis(500));
        }
        if !ready {
            let _ = fs::remove_file(&self.config.maintenance_flag);
            self.maintenance_generation_ns = None;
            return Err(UpdateError::Busy(
                "server did not reach bounded quiescence".into(),
            ));
        }
        self.stopped = true;
        if let Err(error) = self.systemctl("stop") {
            let _ = fs::remove_file(&self.config.maintenance_flag);
            self.maintenance_generation_ns = None;
            return Err(error);
        }
        Ok(())
    }
    fn backup(&mut self, component: Component, _: &Path) -> chill_updates::Result<String> {
        if component == Component::Server {
            let binary = self
                .config
                .component_roots
                .get(&Component::Server)
                .unwrap()
                .join("current/payload");
            let started = OffsetDateTime::now_utc().unix_timestamp();
            let mut command = Command::new(binary);
            command
                .args(["--backup", "--config"])
                .arg(&self.config.server_config)
                .env_clear()
                .env("PATH", "/usr/bin:/bin")
                .current_dir("/usr/lib/chillcenter");
            let uid = self.config.service_uid;
            let gid = self.config.service_gid;
            // SAFETY: these libc calls are async-signal-safe and execute in the
            // child immediately before exec. Supplementary root groups are
            // cleared before gid/uid are irreversibly dropped.
            unsafe {
                command.pre_exec(move || {
                    if libc::setgroups(0, std::ptr::null()) != 0
                        || libc::setgid(gid) != 0
                        || libc::setuid(uid) != 0
                    {
                        return Err(std::io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
            let output = run_bounded_backup(&mut command, Duration::from_secs(120))?;
            if output.len() > 4096 {
                return Err(UpdateError::Invalid(
                    "offline server backup receipt is too large".into(),
                ));
            }
            let line = std::str::from_utf8(&output)
                .map_err(|_| UpdateError::Security("backup receipt is not UTF-8".into()))?
                .trim();
            let path = line
                .strip_prefix("Sauvegarde terminée : ")
                .ok_or_else(|| UpdateError::Security("backup receipt is missing".into()))?;
            return validate_backup_receipt(
                Path::new(path),
                &self.config.backup_root,
                self.config.service_uid,
                started,
            );
        }
        Ok(format!(
            "retained-{}",
            OffsetDateTime::now_utc().unix_timestamp()
        ))
    }
    fn before_switch(&mut self, component: Component) -> chill_updates::Result<()> {
        let acceptance = self
            .acceptance
            .as_ref()
            .ok_or_else(|| UpdateError::Security("missing root acceptance identity".into()))?;
        if acceptance.component != component {
            return Err(UpdateError::Security("acceptance component changed".into()));
        }
        if acceptance.automatic
            && state_store(self.config)?
                .read()?
                .policies
                .get(&component)
                .is_some_and(|policy| policy.mode != PolicyMode::Automatic)
        {
            return Err(UpdateError::Busy(
                "automatic installation was disabled".into(),
            ));
        }
        let trust = AntiRollbackStore::open_root(&self.config.trust_root)?;
        let now = OffsetDateTime::now_utc();
        if trust.pending(component)?.is_none()
            && (now < acceptance.issued || now >= acceptance.expires)
        {
            return Err(UpdateError::Security(
                "release no longer valid at root acceptance".into(),
            ));
        }
        trust.accept_pending(
            component,
            acceptance.sequence,
            &acceptance.digest,
            &acceptance.version,
        )
    }
    fn activate(&mut self, component: Component, _: &Path) -> chill_updates::Result<()> {
        let root = self
            .config
            .component_roots
            .get(&component)
            .ok_or_else(|| UpdateError::Security("component root missing".into()))?;
        let authorization = activation_authorization(self.config, component);
        // The core binds a one-shot token to this live helper PID/starttime,
        // boot, transaction operation and candidate. It deliberately refuses
        // stale authorizations rather than relying on a writable target string.
        authorize_activation(root, &authorization)?;
        let action = if self.stopped { "start" } else { "restart" };
        let restarted = self.systemctl(action);
        let removed = match fs::remove_file(&authorization) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        };
        restarted?;
        removed?;
        self.stopped = false;
        Ok(())
    }
    fn health_check(
        &mut self,
        component: Component,
        installed_root: &Path,
    ) -> chill_updates::Result<()> {
        // Type=simple reports a process before its HTTPS listener is ready.
        // Retry only the fixed, CA-pinned endpoint, within one global budget.
        let deadline = Instant::now() + Duration::from_secs(15);
        let attempt = || -> chill_updates::Result<()> {
            let endpoint = if component == Component::Server {
                &self.config.health_url
            } else {
                &self.config.status_url
            };
            let remaining = deadline.saturating_duration_since(Instant::now());
            let response = self
                .client
                .get(endpoint)
                .timeout(remaining.min(Duration::from_secs(2)))
                .send()?
                .error_for_status()?;
            let mut body = Vec::new();
            response.take(1024 * 1024 + 1).read_to_end(&mut body)?;
            if body.len() > 1024 * 1024 {
                return Err(UpdateError::Security("health response is too large".into()));
            }
            let value: serde_json::Value = parse_strict_json(&body)?;
            let leaf = installed_root
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or_else(|| UpdateError::Security("health slot has invalid name".into()))?;
            let expected = leaf
                .rsplit_once('-')
                .map(|(version, _)| version)
                .ok_or_else(|| UpdateError::Security("health slot has no sequence".into()))?;
            Version::parse(expected)
                .map_err(|_| UpdateError::Security("health slot version is invalid".into()))?;
            let actual = match component {
                Component::Server => value.get("version"),
                Component::Web => value.pointer("/installed/web/version"),
                Component::Themes => value.pointer("/installed/themes/version"),
                Component::Mac => None,
            }
            .and_then(|value| value.as_str());
            if actual != Some(expected) {
                return Err(UpdateError::Incompatible(
                    "health endpoint did not report installed version".into(),
                ));
            }
            Ok(())
        };
        loop {
            match attempt() {
                Ok(()) => return Ok(()),
                Err(error) if Instant::now() >= deadline => return Err(error),
                Err(_) => thread::sleep(
                    Duration::from_millis(250)
                        .min(deadline.saturating_duration_since(Instant::now())),
                ),
            }
        }
    }
    fn end_quiescence(&mut self, _component: Component) -> chill_updates::Result<()> {
        let restarted = if self.stopped {
            self.systemctl("start")
        } else {
            Ok(())
        };
        self.stopped = false;
        let result = match fs::remove_file(&self.config.maintenance_flag) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        };
        self.maintenance_generation_ns = None;
        restarted.and(result)
    }
}

fn os_local_offset() -> chill_updates::Result<UtcOffset> {
    let timestamp = OffsetDateTime::now_utc().unix_timestamp() as libc::time_t;
    let mut local: libc::tm = unsafe { std::mem::zeroed() };
    if unsafe { libc::localtime_r(&timestamp, &mut local) }.is_null() {
        return Err(UpdateError::Invalid(
            "OS local timezone is unavailable".into(),
        ));
    }
    let seconds: i32 = local
        .tm_gmtoff
        .try_into()
        .map_err(|_| UpdateError::Invalid("OS local offset is outside supported range".into()))?;
    UtcOffset::from_whole_seconds(seconds)
        .map_err(|_| UpdateError::Invalid("OS local offset is invalid".into()))
}

fn private_health_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(value) => value.is_private() || value.is_loopback() || value.is_link_local(),
        IpAddr::V6(value) => {
            value.is_unique_local() || value.is_loopback() || value.is_unicast_link_local()
        }
    }
}

#[cfg(test)]
mod helper_tests {
    use super::*;

    #[test]
    fn interrupted_download_never_publishes_a_partial_digest_and_retries_exact_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let payload = b"synthetic complete payload";
        let digest = hex::encode(Sha256::digest(payload));
        let manifest = b"synthetic signed manifest bytes";
        let cache_key = hex::encode(Sha256::digest(manifest));
        for partial in [&b""[..], &b"partial"[..], &payload[..]] {
            let outcome = publish_cached_release(
                temp.path(),
                &cache_key,
                manifest,
                "test signature",
                payload.len() as u64,
                &digest,
                |path| {
                    write_new_sync(path, partial)?;
                    Err(std::io::Error::other("injected interrupted download").into())
                },
            );
            assert!(outcome.is_err());
            assert!(
                !temp.path().join(&cache_key).exists(),
                "partial digest path became visible"
            );
        }
        publish_cached_release(
            temp.path(),
            &cache_key,
            manifest,
            "test signature",
            payload.len() as u64,
            &digest,
            |path| write_new_sync(path, payload),
        )
        .unwrap();
        assert_eq!(
            fs::read(temp.path().join(&cache_key).join("payload")).unwrap(),
            payload
        );
        publish_cached_release(
            temp.path(),
            &cache_key,
            manifest,
            "test signature",
            payload.len() as u64,
            &digest,
            |_| panic!("complete cache must be reused"),
        )
        .unwrap();
        assert_eq!(
            fs::read(temp.path().join(&cache_key).join("manifest.json")).unwrap(),
            manifest
        );
    }

    #[test]
    fn cleanup_only_recognizes_our_bounded_atomic_cache_temporary_names() {
        let id = Uuid::new_v4();
        assert!(matches!(
            abandoned_cache_temporary_kind(&format!(".download-{id}")),
            Some(CacheTemporary::Download)
        ));
        assert!(matches!(
            abandoned_cache_temporary_kind(&format!(".available-{id}.tmp")),
            Some(CacheTemporary::Available)
        ));
        for value in [
            ".download-not-a-uuid",
            ".available-x.tmp",
            ".available-00000000-0000-0000-0000-000000000000",
            "payload",
        ] {
            assert!(abandoned_cache_temporary_kind(value).is_none());
        }
    }

    #[test]
    fn bounded_backup_runner_handles_success_failure_output_and_process_groups() {
        let mut success = Command::new("/bin/sh");
        success.args(["-c", "printf 'receipt\\n'"]);
        assert_eq!(
            run_bounded_backup(&mut success, Duration::from_secs(1)).unwrap(),
            b"receipt\n"
        );

        let mut failure = Command::new("/bin/sh");
        failure.args(["-c", "echo failure >&2; exit 7"]);
        assert!(run_bounded_backup(&mut failure, Duration::from_secs(1)).is_err());

        let mut noisy = Command::new("/bin/sh");
        noisy.args(["-c", "yes output"]);
        assert!(run_bounded_backup(&mut noisy, Duration::from_secs(1)).is_err());

        let mut hung_group = Command::new("/bin/sh");
        hung_group.args(["-c", "sleep 30 & wait"]);
        assert!(run_bounded_backup(&mut hung_group, Duration::from_millis(50)).is_err());
    }
}
