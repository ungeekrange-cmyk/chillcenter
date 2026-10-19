use chill_updates::{
    AntiRollbackStore, ApplyRequest, Component, ExtractionLimits, FIXED_GITHUB_OWNER,
    FIXED_GITHUB_REPOSITORY, GithubReleaseClient, InstallHooks, InstallLayout, IntentKind,
    OperationStatus, PolicyMode, RuntimeCompatibility, StateStore, TransportLimits, UpdateError,
    VerificationContext, apply_staged, due_attempts, parse_and_verify_manifest, parse_strict_json,
    record_retry, validate_root_owned_file,
};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use std::thread;
use std::time::{Duration, UNIX_EPOCH};
use time::{OffsetDateTime, UtcOffset};
use uuid::Uuid;

const DEFAULT_CONFIG: &str = "/etc/chillcenter/updates.json";
const CONFIRM_INTERRUPT: &str = "INTERRUPT_ACTIVE_WORK";

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
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AvailabilityEntry {
    version: Version,
    source_commit: String,
    release_sequence: u64,
    checked_unix: i64,
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
    let config = load_config(&config_path)?;
    match arguments.first().map(String::as_str) {
        Some("inspect") if arguments.len() == 1 => inspect(&config),
        Some("check") if arguments.len() == 2 => {
            perform_check(&config, parse_component(&arguments[1])?).map(|_| ())
        }
        Some("manifest") if arguments.len() == 2 => {
            verify_cached(&config, parse_component(&arguments[1])?).map(|_| ())
        }
        Some("apply") | Some("install") => {
            if arguments.len() < 2 {
                return Err(UpdateError::Invalid("component is required".into()));
            }
            let component = parse_component(&arguments[1])?;
            let first_install = arguments[0] == "install";
            let force = arguments.iter().any(|arg| arg == "--force");
            let confirmed = arguments
                .iter()
                .any(|arg| arg == &format!("--confirm-interrupt={CONFIRM_INTERRUPT}"));
            if force && !confirmed {
                return Err(UpdateError::Security(format!(
                    "--force requires --confirm-interrupt={CONFIRM_INTERRUPT}"
                )));
            }
            if arguments
                .iter()
                .skip(2)
                .any(|arg| arg != "--force" && !arg.starts_with("--confirm-interrupt="))
            {
                return Err(UpdateError::Invalid("unknown apply option".into()));
            }
            perform_apply(&config, component, first_install, force)
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
        "chill-update {}\n\nUsage:\n  chill-update [--config PATH] inspect\n  chill-update [--config PATH] check COMPONENT\n  chill-update [--config PATH] manifest COMPONENT\n  chill-update [--config PATH] policy COMPONENT MODE HOUR MINUTE EXPECTED_REVISION\n  chill-update [--config PATH] intent COMPONENT check|install\n  chill-update [--config PATH] tick\n  chill-update [--config PATH] apply COMPONENT [--force --confirm-interrupt={CONFIRM_INTERRUPT}]\n  chill-update [--config PATH] install COMPONENT\n\nCOMPONENT is server, web, or themes on Linux. `install` is first-install only.\nNo command accepts a URL, repository, key, shell command, tag, or destination path.",
        env!("CARGO_PKG_VERSION")
    );
}

fn load_config(path: &Path) -> chill_updates::Result<RootConfig> {
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
    for path in [
        &config.public_key,
        &config.server_config,
        &config.health_ca_certificate,
    ] {
        validate_root_owned_file(path, 1024 * 1024)?;
    }
    for path in [
        &config.state_root,
        &config.trust_root,
        &config.cache_root,
        &config.backup_root,
    ] {
        if !path.is_absolute() {
            return Err(UpdateError::Security(
                "configured roots must be absolute".into(),
            ));
        }
    }
    if !config.maintenance_flag.is_absolute() || !config.quiescence_status.is_absolute() {
        return Err(UpdateError::Security(
            "runtime paths must be absolute".into(),
        ));
    }
    let url = reqwest::Url::parse(&config.health_url)
        .map_err(|_| UpdateError::Invalid("invalid health URL".into()))?;
    if url.scheme() != "https"
        || !matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"))
    {
        return Err(UpdateError::Security(
            "health URL must be loopback HTTPS".into(),
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
        return Ok(if operation.result_code.as_deref() == Some("no_update") {
            CheckResult::NoUpdate
        } else {
            CheckResult::Available
        });
    }
    store.transition(
        operation.id,
        OperationStatus::Pending,
        OperationStatus::Running,
        None,
        now,
    )?;
    let outcome = (|| {
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
        let directory = config
            .cache_root
            .join(component.as_str())
            .join(operation.id.to_string());
        fs::create_dir_all(&directory)?;
        write_new_sync(&directory.join("manifest.json"), &manifest_bytes)?;
        write_new_sync(
            &directory.join("manifest.json.minisig"),
            signature.as_bytes(),
        )?;
        client.download_verified(
            &verified.selected_asset.url,
            &directory.join("payload"),
            verified.selected_asset.byte_length,
            &verified.selected_asset.sha256,
        )?;
        write_available(
            config,
            &AvailableReceipt {
                schema_version: 1,
                operation_id: operation.id,
                component,
                target: target.clone(),
                release_tag: verified.manifest.release_tag,
            },
        )?;
        publish_availability(
            config,
            component,
            AvailabilityEntry {
                version: verified.manifest.version,
                source_commit: verified.manifest.source_commit,
                release_sequence: verified.manifest.release_sequence,
                checked_unix: now.unix_timestamp(),
            },
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

fn verify_cached(
    config: &RootConfig,
    component: Component,
) -> chill_updates::Result<chill_updates::VerifiedManifest> {
    let receipt = read_available(config, component)?;
    let directory = config
        .cache_root
        .join(component.as_str())
        .join(receipt.operation_id.to_string());
    let manifest = read_bounded(&directory.join("manifest.json"), 1024 * 1024)?;
    let signature = String::from_utf8(read_bounded(
        &directory.join("manifest.json.minisig"),
        16 * 1024,
    )?)
    .map_err(|_| UpdateError::Security("signature is not UTF-8".into()))?;
    let public_key = String::from_utf8(read_bounded(&config.public_key, 16 * 1024)?)
        .map_err(|_| UpdateError::Security("public key is not UTF-8".into()))?;
    let installed = installed_version(config, component)?;
    let highest = AntiRollbackStore::open_root(&config.trust_root)?.highest(component)?;
    let context = verification_context(
        config,
        component,
        &receipt.target,
        &installed,
        highest,
        OffsetDateTime::now_utc(),
    );
    let verified = parse_and_verify_manifest(&manifest, &signature, &public_key, &context)?;
    if verified.manifest.release_tag != receipt.release_tag {
        return Err(UpdateError::Security(
            "available receipt does not match manifest".into(),
        ));
    }
    Ok(verified)
}

fn perform_apply(
    config: &RootConfig,
    component: Component,
    first_install: bool,
    force: bool,
) -> chill_updates::Result<()> {
    require_linux_component(component)?;
    let receipt = read_available(config, component)?;
    let verified = verify_cached(config, component)?;
    let store = state_store(config)?;
    let operation =
        store.admit_intent(component, IntentKind::Install, OffsetDateTime::now_utc())?;
    if operation.status != OperationStatus::Pending {
        return Err(UpdateError::Busy(format!(
            "install operation {} is already {:?}",
            operation.id, operation.status
        )));
    }
    store.transition(
        operation.id,
        OperationStatus::Pending,
        OperationStatus::Running,
        None,
        OffsetDateTime::now_utc(),
    )?;
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
        .join(receipt.operation_id.to_string())
        .join("payload");
    let mut hooks = LinuxHooks::new(config, verified.manifest.version.clone())?;
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
            AntiRollbackStore::open_root(&config.trust_root)?
                .advance(component, verified.manifest.release_sequence)?;
            store.transition(
                operation.id,
                OperationStatus::Running,
                OperationStatus::Succeeded,
                Some("installed"),
                OffsetDateTime::now_utc(),
            )?;
            println!(
                "installed={} backup={}",
                result.installed.display(),
                result.backup_id
            );
            Ok(())
        }
        Err(error) => {
            let _ = store.transition(
                operation.id,
                OperationStatus::Running,
                OperationStatus::Failed,
                Some("apply_failed"),
                OffsetDateTime::now_utc(),
            );
            Err(error)
        }
    }
}

fn tick(config: &RootConfig) -> chill_updates::Result<()> {
    let offset = os_local_offset()?;
    let store = state_store(config)?;
    let pending = store
        .read()?
        .operations
        .into_iter()
        .filter(|operation| operation.status == OperationStatus::Pending)
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
        if let Err(error) = perform_apply(config, operation.component, false, false) {
            eprintln!(
                "pending {} install failed: {error}",
                operation.component.as_str()
            );
        }
    }
    let attempts =
        store.update_scheduler(|state| due_attempts(state, OffsetDateTime::now_utc(), offset))?;
    let policies = store.read()?.policies;
    for attempt in attempts {
        let check = perform_check(config, attempt.component);
        if matches!(check, Ok(CheckResult::Available))
            && policies
                .get(&attempt.component)
                .is_some_and(|policy| policy.mode == PolicyMode::Automatic)
        {
            perform_apply(config, attempt.component, false, false)?;
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

fn available_path(config: &RootConfig, component: Component) -> PathBuf {
    config
        .cache_root
        .join(component.as_str())
        .join("available.json")
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
    {
        return Err(UpdateError::Security("available receipt is invalid".into()));
    }
    Ok(receipt)
}

fn publish_availability(
    config: &RootConfig,
    component: Component,
    entry: AvailabilityEntry,
) -> chill_updates::Result<()> {
    // UI-only snapshot. It is never consulted by signature, sequence, path, or
    // install decisions; those are revalidated from the root-owned cache/trust store.
    let path = config.state_root.join("available.json");
    let mut entries: BTreeMap<Component, AvailabilityEntry> = match read_bounded(&path, 64 * 1024) {
        Ok(bytes) => parse_strict_json(&bytes)?,
        Err(UpdateError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            BTreeMap::new()
        }
        Err(error) => return Err(error),
    };
    entries.insert(component, entry);
    if entries.len() > 4 {
        return Err(UpdateError::Security(
            "availability snapshot is unbounded".into(),
        ));
    }
    let bytes = serde_json::to_vec_pretty(&entries)?;
    let temporary = config
        .state_root
        .join(format!(".available-{}.tmp", Uuid::new_v4()));
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
    File::open(&config.state_root)?.sync_all()?;
    Ok(())
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

struct LinuxHooks<'a> {
    config: &'a RootConfig,
    expected_version: Version,
    client: reqwest::blocking::Client,
    stopped: bool,
    maintenance_generation_ns: Option<String>,
}

impl<'a> LinuxHooks<'a> {
    fn new(config: &'a RootConfig, expected_version: Version) -> chill_updates::Result<Self> {
        let certificate = reqwest::Certificate::from_pem(&read_bounded(
            &config.health_ca_certificate,
            1024 * 1024,
        )?)?;
        let client = reqwest::blocking::Client::builder()
            .add_root_certificate(certificate)
            .timeout(Duration::from_secs(10))
            .build()?;
        Ok(Self {
            config,
            expected_version,
            client,
            stopped: false,
            maintenance_generation_ns: None,
        })
    }
    fn systemctl(&self, action: &str) -> chill_updates::Result<()> {
        let status = Command::new("/usr/bin/systemctl")
            .args([action, self.config.service_name.as_str()])
            .status()?;
        if !status.success() {
            return Err(UpdateError::Invalid(format!("systemctl {action} failed")));
        }
        Ok(())
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
        if component == Component::Server {
            if let Err(error) = self.systemctl("stop") {
                let _ = fs::remove_file(&self.config.maintenance_flag);
                self.maintenance_generation_ns = None;
                return Err(error);
            }
            self.stopped = true;
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
            let status = Command::new(binary)
                .args(["--backup", "--config"])
                .arg(&self.config.server_config)
                .uid(self.config.service_uid)
                .gid(self.config.service_gid)
                .status()?;
            if !status.success() {
                return Err(UpdateError::Invalid("offline server backup failed".into()));
            }
        }
        Ok(format!(
            "retained-{}",
            OffsetDateTime::now_utc().unix_timestamp()
        ))
    }
    fn activate(&mut self, component: Component, _: &Path) -> chill_updates::Result<()> {
        if component == Component::Server {
            self.systemctl("restart")?;
            self.stopped = false;
        }
        Ok(())
    }
    fn health_check(&mut self, component: Component, _: &Path) -> chill_updates::Result<()> {
        let response = self
            .client
            .get(&self.config.health_url)
            .send()?
            .error_for_status()?;
        let mut body = Vec::new();
        response.take(1024 * 1024 + 1).read_to_end(&mut body)?;
        if body.len() > 1024 * 1024 {
            return Err(UpdateError::Security("health response is too large".into()));
        }
        let value: serde_json::Value = parse_strict_json(&body)?;
        let expected = self.expected_version.to_string();
        let actual = match component {
            Component::Server => value.get("version"),
            Component::Web => value.pointer("/installed/web"),
            Component::Themes => value.pointer("/installed/themes"),
            Component::Mac => None,
        }
        .and_then(|value| value.as_str());
        if actual != Some(expected.as_str()) {
            return Err(UpdateError::Incompatible(
                "health endpoint did not report installed version".into(),
            ));
        }
        Ok(())
    }
    fn end_quiescence(&mut self, component: Component) -> chill_updates::Result<()> {
        if component == Component::Server && self.stopped {
            self.systemctl("start")?;
            self.stopped = false;
        }
        let result = match fs::remove_file(&self.config.maintenance_flag) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        };
        self.maintenance_generation_ns = None;
        result
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
