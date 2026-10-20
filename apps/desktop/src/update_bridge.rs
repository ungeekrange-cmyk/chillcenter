//! Device-local update worker; neither catalogue locks nor profile state are
//! held across GitHub requests. Install URLs/paths are never supplied by JS.
use base64::{Engine, engine::general_purpose::STANDARD};
use chill_client::ui::UiError;
use chill_updates::{
    Component, GithubReleaseClient, IntentKind, OperationStatus, PolicyMode, RuntimeCompatibility,
    StateStore, TransportLimits, VerificationContext, VerifiedManifest,
};
use semver::Version;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tauri::Manager;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_updater::UpdaterExt;
const PUBLIC_KEY: &str = include_str!("../../../updates/release.pub");
const KEY_ID: &str = "18FD790B48C202F8";
#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum UpdateCommand {
    Status,
    Check {
        component: Component,
    },
    Install {
        component: Component,
    },
    Policy {
        component: Component,
        mode: PolicyMode,
        hour: u8,
        minute: u8,
        expected_revision: u64,
    },
    ServerStatus,
    ServerCheck {
        component: Component,
    },
    ServerInstall {
        component: Component,
        confirmed: bool,
    },
    ServerPolicy {
        component: Component,
        mode: PolicyMode,
        hour: u8,
        minute: u8,
        expected_revision: u64,
    },
}
pub fn error(message: impl Into<String>, code: &str) -> UiError {
    UiError {
        message: message.into(),
        status: None,
        code: code.into(),
    }
}

fn macos_post_consent_rendering_guard(rendering: &AtomicBool) -> Result<(), UiError> {
    if rendering.load(Ordering::Acquire) {
        return Err(error(
            "Une lecture a commencé ; relancez l’installation après sa fin.",
            "UPDATE_BUSY",
        ));
    }
    Ok(())
}

/// Native confirmation is deliberately owned by the Rust orchestration path.
/// The caller must still perform its pre-dialog guard before fetching any
/// metadata; this helper preserves the second guard immediately after consent.
pub(super) async fn confirm_macos_install(
    app: &tauri::AppHandle,
    version: &Version,
    rendering: &AtomicBool,
) -> Result<(), UiError> {
    let (answer, receive) = tokio::sync::oneshot::channel();
    app.dialog().message(format!("Installer Chill Center {version} et redémarrer maintenant ?\nInstall Chill Center {version} and restart now?"))
        .title("Chill Center").kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom("Installer maintenant / Install now".into(),"Annuler / Cancel".into()))
        .show(move |confirmed|{let _=answer.send(confirmed);});
    if !receive.await.unwrap_or(false) {
        return Err(error("Installation annulée.", "UPDATE_CANCELLED"));
    }
    macos_post_consent_rendering_guard(rendering)
}
pub struct UpdateHost {
    pub directory: PathBuf,
    active: AtomicBool,
    available: Mutex<BTreeMap<Component, AvailableRelease>>,
    note: Mutex<String>,
    pending_theme: AtomicBool,
}
#[derive(Clone)]
struct AvailableRelease {
    verified: VerifiedManifest,
    // Preserve the signed bytes, not just their parsed interpretation. Time,
    // runtime compatibility and the replay ledger are checked again at use.
    bytes: Vec<u8>,
    signature: String,
}
struct DownloadedAsset(PathBuf);
struct InstallLease(std::fs::File);

const PENDING_SCHEMA_VERSION: u32 = 1;
const MAX_PENDING_MANIFEST_BYTES: usize = 64 * 1024;
const MAX_PENDING_SIGNATURE_BYTES: usize = 8 * 1024;

/// A device-local acceptance receipt is deliberately separate from the generic
/// scheduler state.  It is the only exception to the monotonic sequence rule:
/// it admits the exact signed manifest which failed after acceptance, never a
/// different same-sequence release.
#[derive(Clone, Debug, Deserialize, serde::Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct PendingRelease {
    sequence: u64,
    version: Version,
    source_commit: String,
    accepted_unix: i64,
    manifest_sha256: String,
    manifest_base64: String,
    signature: String,
}

#[derive(Default, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct PendingState {
    schema_version: u32,
    releases: BTreeMap<Component, PendingRelease>,
}

struct PendingStore {
    root: PathBuf,
    state: PathBuf,
    lock: PathBuf,
}

impl PendingStore {
    fn open(directory: &std::path::Path) -> Result<Self, UiError> {
        use std::os::unix::fs::{DirBuilderExt, MetadataExt};
        let root = directory.join("updates/local-pending");
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&root)
            .map_err(|_| {
                error(
                    "Stockage de reprise indisponible.",
                    "UPDATE_STATE_UNAVAILABLE",
                )
            })?;
        let metadata = fs::symlink_metadata(&root).map_err(|_| {
            error(
                "Stockage de reprise indisponible.",
                "UPDATE_STATE_UNAVAILABLE",
            )
        })?;
        if !metadata.is_dir()
            || metadata.file_type().is_symlink()
            || metadata.uid() != unsafe { libc::geteuid() }
            || metadata.mode() & 0o077 != 0
        {
            return Err(error(
                "Stockage de reprise indisponible.",
                "UPDATE_STATE_UNAVAILABLE",
            ));
        }
        Ok(Self {
            state: root.join("pending.json"),
            lock: root.join("pending.lock"),
            root,
        })
    }

    fn with_lock<T>(
        &self,
        action: impl FnOnce(&mut PendingState) -> Result<(T, bool), UiError>,
    ) -> Result<T, UiError> {
        use std::os::{
            fd::AsRawFd,
            unix::fs::{MetadataExt, OpenOptionsExt},
        };
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&self.lock)
            .map_err(|_| {
                error(
                    "Verrou de reprise indisponible.",
                    "UPDATE_STATE_UNAVAILABLE",
                )
            })?;
        let metadata = lock.metadata().map_err(|_| {
            error(
                "Verrou de reprise indisponible.",
                "UPDATE_STATE_UNAVAILABLE",
            )
        })?;
        if !metadata.is_file()
            || metadata.uid() != unsafe { libc::geteuid() }
            || metadata.mode() & 0o077 != 0
            || unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX) } != 0
        {
            return Err(error(
                "Verrou de reprise indisponible.",
                "UPDATE_STATE_UNAVAILABLE",
            ));
        }
        let mut state = self.load()?;
        let (value, changed) = action(&mut state)?;
        if changed {
            self.save(&state)?;
        }
        unsafe {
            libc::flock(lock.as_raw_fd(), libc::LOCK_UN);
        }
        Ok(value)
    }

    fn load(&self) -> Result<PendingState, UiError> {
        use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
        match OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&self.state)
        {
            Ok(mut file) => {
                let metadata = file
                    .metadata()
                    .map_err(|_| error("Reprise indisponible.", "UPDATE_STATE_UNAVAILABLE"))?;
                if !metadata.is_file()
                    || metadata.uid() != unsafe { libc::geteuid() }
                    || metadata.mode() & 0o077 != 0
                    || metadata.len() > 128 * 1024
                {
                    return Err(error("Reprise indisponible.", "UPDATE_STATE_UNAVAILABLE"));
                }
                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes)
                    .map_err(|_| error("Reprise indisponible.", "UPDATE_STATE_UNAVAILABLE"))?;
                let state: PendingState = serde_json::from_slice(&bytes)
                    .map_err(|_| error("Reprise indisponible.", "UPDATE_STATE_UNAVAILABLE"))?;
                Self::validate(&state)?;
                Ok(state)
            }
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(PendingState {
                schema_version: PENDING_SCHEMA_VERSION,
                releases: BTreeMap::new(),
            }),
            Err(_) => Err(error("Reprise indisponible.", "UPDATE_STATE_UNAVAILABLE")),
        }
    }

    fn save(&self, state: &PendingState) -> Result<(), UiError> {
        use std::os::unix::fs::OpenOptionsExt;
        Self::validate(state)?;
        let bytes = serde_json::to_vec(state)
            .map_err(|_| error("Reprise indisponible.", "UPDATE_STATE_UNAVAILABLE"))?;
        let temporary = self
            .root
            .join(format!(".pending-{}.tmp", uuid::Uuid::new_v4()));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&temporary)
            .map_err(|_| error("Reprise indisponible.", "UPDATE_STATE_UNAVAILABLE"))?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| error("Reprise indisponible.", "UPDATE_STATE_UNAVAILABLE"))?;
        fs::rename(&temporary, &self.state)
            .map_err(|_| error("Reprise indisponible.", "UPDATE_STATE_UNAVAILABLE"))?;
        File::open(&self.root)
            .and_then(|file| file.sync_all())
            .map_err(|_| error("Reprise indisponible.", "UPDATE_STATE_UNAVAILABLE"))
    }

    fn validate(state: &PendingState) -> Result<(), UiError> {
        if state.schema_version != PENDING_SCHEMA_VERSION
            || state.releases.len() > 2
            || state
                .releases
                .keys()
                .any(|component| !matches!(component, Component::Mac | Component::Themes))
        {
            return Err(error("Reprise indisponible.", "UPDATE_STATE_UNAVAILABLE"));
        }
        for receipt in state.releases.values() {
            if receipt.sequence == 0
                || receipt.accepted_unix <= 0
                || receipt.source_commit.len() != 40
                || !receipt
                    .source_commit
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit())
                || receipt.manifest_sha256.len() != 64
                || !receipt
                    .manifest_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit())
                || receipt.signature.len() > MAX_PENDING_SIGNATURE_BYTES
            {
                return Err(error("Reprise indisponible.", "UPDATE_STATE_UNAVAILABLE"));
            }
            let bytes = STANDARD
                .decode(&receipt.manifest_base64)
                .map_err(|_| error("Reprise indisponible.", "UPDATE_STATE_UNAVAILABLE"))?;
            if bytes.is_empty()
                || bytes.len() > MAX_PENDING_MANIFEST_BYTES
                || hex_digest(&bytes) != receipt.manifest_sha256
            {
                return Err(error("Reprise indisponible.", "UPDATE_STATE_UNAVAILABLE"));
            }
        }
        Ok(())
    }

    fn get(&self, component: Component) -> Result<Option<PendingRelease>, UiError> {
        self.with_lock(|state| Ok((state.releases.get(&component).cloned(), false)))
    }

    fn accept(
        &self,
        component: Component,
        verified: &VerifiedManifest,
        bytes: &[u8],
        signature: &str,
    ) -> Result<(), UiError> {
        let mut receipt = PendingRelease {
            sequence: verified.manifest.release_sequence,
            version: verified.manifest.version.clone(),
            source_commit: verified.manifest.source_commit.clone(),
            accepted_unix: time::OffsetDateTime::now_utc().unix_timestamp(),
            manifest_sha256: hex_digest(bytes),
            manifest_base64: STANDARD.encode(bytes),
            signature: signature.to_owned(),
        };
        Self::validate(&PendingState {
            schema_version: PENDING_SCHEMA_VERSION,
            releases: BTreeMap::from([(component, receipt.clone())]),
        })?;
        self.with_lock(|state| {
            if let Some(existing) = state.releases.get(&component) {
                if existing.sequence == receipt.sequence {
                    receipt.accepted_unix = existing.accepted_unix;
                }
                if existing.sequence > receipt.sequence
                    || (existing.sequence == receipt.sequence && existing != &receipt)
                {
                    return Err(error(
                        "Une autre mise à jour acceptée doit être reprise exactement.",
                        "UPDATE_RECOVERY_REQUIRED",
                    ));
                }
                if existing == &receipt {
                    return Ok(((), false));
                }
            }
            let accepted = time::OffsetDateTime::from_unix_timestamp(receipt.accepted_unix)
                .map_err(|_| error("Reçu invalide.", "UPDATE_RECHECK_REQUIRED"))?;
            let parse_time = |text: &str| {
                time::OffsetDateTime::parse(text, &time::format_description::well_known::Rfc3339)
                    .map_err(|_| error("Publication invalide.", "UPDATE_RECHECK_REQUIRED"))
            };
            if accepted < parse_time(&verified.manifest.issued_at)?
                || accepted >= parse_time(&verified.manifest.expires_at)?
            {
                return Err(error(
                    "Publication expirée : vérifiez à nouveau.",
                    "UPDATE_RECHECK_REQUIRED",
                ));
            }
            state.releases.insert(component, receipt);
            Ok(((), true))
        })
    }

    fn commit(
        &self,
        component: Component,
        verified: &VerifiedManifest,
        bytes: &[u8],
    ) -> Result<(), UiError> {
        let digest = hex_digest(bytes);
        self.with_lock(|state| {
            let Some(receipt) = state.releases.get(&component) else {
                return Err(error(
                    "Acceptation de reprise absente.",
                    "UPDATE_STATE_UNAVAILABLE",
                ));
            };
            if receipt.sequence != verified.manifest.release_sequence
                || receipt.version != verified.manifest.version
                || receipt.manifest_sha256 != digest
            {
                return Err(error(
                    "Identité de mise à jour modifiée.",
                    "UPDATE_RECHECK_REQUIRED",
                ));
            }
            state.releases.remove(&component);
            Ok(((), true))
        })
    }
}

fn hex_digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
impl InstallLease {
    fn acquire(directory: &std::path::Path) -> Result<Self, UiError> {
        use std::os::{
            fd::AsRawFd,
            unix::fs::{MetadataExt, OpenOptionsExt},
        };
        let root = directory.join("updates");
        std::fs::create_dir_all(&root)
            .map_err(|_| error("Stockage indisponible.", "UPDATE_STATE_UNAVAILABLE"))?;
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(root.join("install.lock"))
            .map_err(|_| error("Verrou indisponible.", "UPDATE_BUSY"))?;
        let metadata = file
            .metadata()
            .map_err(|_| error("Verrou indisponible.", "UPDATE_BUSY"))?;
        if !metadata.is_file()
            || metadata.uid() != unsafe { libc::geteuid() }
            || metadata.mode() & 0o022 != 0
            || unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0
        {
            return Err(error(
                "Une autre instance installe déjà une mise à jour.",
                "UPDATE_BUSY",
            ));
        }
        Ok(Self(file))
    }
}
impl Drop for InstallLease {
    fn drop(&mut self) {
        use std::os::fd::AsRawFd;
        unsafe {
            libc::flock(self.0.as_raw_fd(), libc::LOCK_UN);
        }
    }
}
impl Drop for DownloadedAsset {
    fn drop(&mut self) {
        // This is one exact generated cache file, never a recursive removal.
        let _ = std::fs::remove_file(&self.0);
    }
}
impl UpdateHost {
    pub fn new(directory: PathBuf) -> Self {
        let interrupted = directory.join("updates/themes/transaction.json").exists();
        let recovery = if interrupted {
            InstallLease::acquire(&directory).and_then(|_lease| {
                chill_updates::recover_interrupted(&directory.join("updates/themes"), None).map_err(
                    |_| {
                        error(
                            "Récupération des ambiances requise.",
                            "THEME_RECOVERY_REQUIRED",
                        )
                    },
                )
            })
        } else {
            Ok(chill_updates::RecoveryResult::None)
        };
        Self {
            directory,
            active: AtomicBool::new(false),
            available: Mutex::new(BTreeMap::new()),
            note: Mutex::new(recovery.err().map_or(String::new(), |e| e.code)),
            pending_theme: AtomicBool::new(false),
        }
    }
    fn store(&self) -> Result<StateStore, UiError> {
        StateStore::open(self.directory.join("updates/local-state")).map_err(|_| {
            error(
                "Réglages de mise à jour indisponibles.",
                "UPDATE_STATE_UNAVAILABLE",
            )
        })
    }
    pub fn snapshot(&self) -> Result<Value, UiError> {
        let state = self.store()?.read().map_err(|_| {
            error(
                "État de mise à jour indisponible.",
                "UPDATE_STATE_UNAVAILABLE",
            )
        })?;
        let available=self.available.lock().map_err(|_|error("Mise à jour occupée.","UPDATE_BUSY"))?.iter().map(|(component,release)|(component.as_str().to_owned(),json!({"version":release.verified.manifest.version.to_string(),"source_commit":release.verified.manifest.source_commit}))).collect::<serde_json::Map<_,_>>();
        Ok(
            json!({"installed":{"mac":{"version":env!("CARGO_PKG_VERSION")},"themes":{"version":self.theme_version()},"api":chill_client::API_VERSION},"policies":state.policies,"operations":state.operations,"available":available,"busy":self.active.load(Ordering::Acquire),"timezone":local_offset().to_string(),"note":self.note.lock().map(|s|s.clone()).unwrap_or_default()}),
        )
    }
    fn theme_version(&self) -> String {
        let path = self.directory.join("updates/themes/current/manifest.json");
        std::fs::metadata(&path)
            .ok()
            .filter(|m| m.len() <= 64 * 1024)
            .and_then(|_| std::fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
            .and_then(|v| v["version"].as_str().map(str::to_owned))
            .unwrap_or_else(|| "0.7.0".into())
    }
    pub fn policy(
        &self,
        component: Component,
        mode: PolicyMode,
        hour: u8,
        minute: u8,
        revision: u64,
    ) -> Result<Value, UiError> {
        if !matches!(component, Component::Mac | Component::Themes)
            || (component == Component::Mac && mode == PolicyMode::Automatic)
        {
            return Err(error(
                "Sur Mac, l’installation nécessite toujours votre confirmation.",
                "UPDATE_POLICY_INVALID",
            ));
        }
        self.store()?
            .compare_and_set_policy(component, revision, mode, hour, minute)
            .map(|p| json!({"policy":p}))
            .map_err(|_| {
                error(
                    "Le réglage a changé ailleurs. Rechargez les valeurs puis réessayez.",
                    "UPDATE_POLICY_CHANGED",
                )
            })
    }
    pub fn check(self: &Arc<Self>, component: Component) -> Result<Value, UiError> {
        if !matches!(component, Component::Mac | Component::Themes) {
            return Err(error(
                "Composant local invalide.",
                "UPDATE_COMPONENT_INVALID",
            ));
        }
        let store = self.store()?;
        if self.active.swap(true, Ordering::AcqRel) {
            return self.snapshot();
        }
        let operation = match store.admit_intent(
            component,
            IntentKind::Check,
            time::OffsetDateTime::now_utc(),
        ) {
            Ok(op) => op,
            Err(_) => {
                self.active.store(false, Ordering::Release);
                return Err(error(
                    "Vérification déjà effectuée. Consultez son résultat.",
                    "UPDATE_RATE_LIMIT",
                ));
            }
        };
        if operation.status != OperationStatus::Pending {
            self.active.store(false, Ordering::Release);
            return self.snapshot();
        }
        let host = self.clone();
        let id = operation.id;
        tauri::async_runtime::spawn_blocking(move || {
            let store = host.store();
            if let Ok(store) = &store {
                let _ = store.transition(
                    id,
                    operation.status,
                    OperationStatus::Running,
                    None,
                    time::OffsetDateTime::now_utc(),
                );
            }
            let result = host.fetch(component);
            let (status, code) = match result {
                Ok(Some(manifest)) => {
                    if let Ok(mut available) = host.available.lock() {
                        available.insert(component, manifest);
                    }
                    (OperationStatus::Succeeded, "AVAILABLE")
                }
                Ok(None) => {
                    if let Ok(mut available) = host.available.lock() {
                        available.remove(&component);
                    }
                    (OperationStatus::Succeeded, "NO_UPDATE")
                }
                Err(_) => (OperationStatus::Failed, "CHECK_FAILED"),
            };
            if let Ok(store) = store {
                let _ = store.transition(
                    id,
                    OperationStatus::Running,
                    status,
                    Some(code),
                    time::OffsetDateTime::now_utc(),
                );
            }
            if let Ok(mut note) = host.note.lock() {
                *note = code.into();
            }
            host.active.store(false, Ordering::Release);
        });
        Ok(json!({"operation":operation}))
    }
    fn fetch(&self, component: Component) -> chill_updates::Result<Option<AvailableRelease>> {
        let client = GithubReleaseClient::new(TransportLimits {
            max_asset_bytes: 256 * 1024 * 1024,
            ..Default::default()
        })?;
        let Some(candidate) = client.find_latest(component)? else {
            return Ok(None);
        };
        let installed = Version::parse(&if component == Component::Mac {
            env!("CARGO_PKG_VERSION").into()
        } else {
            self.theme_version()
        })
        .map_err(|_| chill_updates::UpdateError::Invalid("installed version".into()))?;
        if candidate.version <= installed {
            return Ok(None);
        }
        let (bytes, signature) = client.download_manifest(&candidate)?;
        let verified = self.verify_current(component, &bytes, &signature)?;
        Ok(Some(AvailableRelease {
            verified,
            bytes,
            signature,
        }))
    }
    fn verify_current(
        &self,
        component: Component,
        bytes: &[u8],
        signature: &str,
    ) -> chill_updates::Result<VerifiedManifest> {
        self.verify_at(component, bytes, signature, time::OffsetDateTime::now_utc())
    }
    fn verify_install(
        &self,
        component: Component,
        bytes: &[u8],
        signature: &str,
    ) -> chill_updates::Result<VerifiedManifest> {
        self.verify_install_using_key(component, bytes, signature, PUBLIC_KEY, KEY_ID)
    }
    /// Kept private so production always uses the embedded release key.  Tests
    /// use this one seam to exercise real Minisign receipts made with an
    /// ephemeral key, rather than depending on a production signing key.
    fn verify_install_using_key(
        &self,
        component: Component,
        bytes: &[u8],
        signature: &str,
        public_key: &str,
        key_id: &str,
    ) -> chill_updates::Result<VerifiedManifest> {
        let pending = PendingStore::open(&self.directory)
            .and_then(|store| store.get(component))
            .map_err(|_| {
                chill_updates::UpdateError::Security("pending receipt unavailable".into())
            })?;
        let now = time::OffsetDateTime::now_utc();
        if let Some(receipt) = pending
            && receipt.manifest_sha256 == hex_digest(bytes)
        {
            let accepted = time::OffsetDateTime::from_unix_timestamp(receipt.accepted_unix)
                .map_err(|_| {
                    chill_updates::UpdateError::Security("invalid acceptance time".into())
                })?;
            if accepted > now {
                return Err(chill_updates::UpdateError::Security(
                    "acceptance time is in the future".into(),
                ));
            }
            // Continue this exact previously-valid transaction. Actual Mac
            // installation still requires a new native confirmation each time.
            return self
                .verify_at_using_key(component, bytes, signature, accepted, public_key, key_id);
        }
        self.verify_at_using_key(component, bytes, signature, now, public_key, key_id)
    }
    fn verify_at(
        &self,
        component: Component,
        bytes: &[u8],
        signature: &str,
        now: time::OffsetDateTime,
    ) -> chill_updates::Result<VerifiedManifest> {
        self.verify_at_using_key(component, bytes, signature, now, PUBLIC_KEY, KEY_ID)
    }
    fn verify_at_using_key(
        &self,
        component: Component,
        bytes: &[u8],
        signature: &str,
        now: time::OffsetDateTime,
        public_key: &str,
        key_id: &str,
    ) -> chill_updates::Result<VerifiedManifest> {
        let installed = Version::parse(&if component == Component::Mac {
            env!("CARGO_PKG_VERSION").into()
        } else {
            self.theme_version()
        })
        .map_err(|_| chill_updates::UpdateError::Invalid("installed version".into()))?;
        let pending = PendingStore::open(&self.directory)
            .map_err(|_| {
                chill_updates::UpdateError::Security("desktop pending receipt unavailable".into())
            })?
            .get(component)
            .map_err(|_| {
                chill_updates::UpdateError::Security("desktop pending receipt unavailable".into())
            })?;
        let digest = hex_digest(bytes);
        let highest = StateStore::open(self.directory.join("updates/local-state"))?
            .read()?
            .components
            .get(&component)
            .map_or(0, |s| s.highest_sequence);
        let exact_pending = pending
            .as_ref()
            .is_some_and(|receipt| receipt.manifest_sha256 == digest);
        let installed = if let Some(receipt) = &pending {
            // A pending receipt is an exact-identity recovery exception, not a
            // general same-sequence exception. Rechecking the persisted bytes
            // below also catches corruption between acceptance and retry.
            if exact_pending && installed == receipt.version {
                Version::new(0, 0, 0)
            } else {
                installed
            }
        } else {
            installed
        };
        let highest = if let Some(receipt) = pending {
            if exact_pending {
                if highest <= receipt.sequence {
                    receipt.sequence.saturating_sub(1)
                } else {
                    highest
                }
            } else {
                highest.max(receipt.sequence)
            }
        } else {
            highest
        };
        let context = VerificationContext {
            now,
            expected_key_id: key_id,
            component,
            target: if component == Component::Mac {
                "darwin-aarch64"
            } else {
                "universal"
            },
            installed_version: &installed,
            highest_sequence: highest,
            runtime: RuntimeCompatibility {
                api_version: Some(Version::new(2026, 10, 14)),
                server_version: None,
                database_schema: Some(3),
                renderer_version: Some(1),
                theme_format: Some(1),
            },
        };
        let verified =
            chill_updates::parse_and_verify_manifest(bytes, signature, public_key, &context)?;
        if exact_pending
            && let Some(receipt) = PendingStore::open(&self.directory)
                .map_err(|_| {
                    chill_updates::UpdateError::Security(
                        "desktop pending receipt unavailable".into(),
                    )
                })?
                .get(component)
                .map_err(|_| {
                    chill_updates::UpdateError::Security(
                        "desktop pending receipt unavailable".into(),
                    )
                })?
            && (receipt.sequence != verified.manifest.release_sequence
                || receipt.version != verified.manifest.version
                || receipt.source_commit != verified.manifest.source_commit
                || receipt.manifest_sha256 != digest)
        {
            return Err(chill_updates::UpdateError::Security(
                "accepted manifest fields changed".into(),
            ));
        }
        Ok(verified)
    }

    fn release_for_install(&self, component: Component) -> Result<AvailableRelease, UiError> {
        if let Some(release) = self
            .available
            .lock()
            .map_err(|_| error("Mise à jour occupée.", "UPDATE_BUSY"))?
            .get(&component)
            .cloned()
        {
            return Ok(release);
        }
        let Some(receipt) = PendingStore::open(&self.directory)?.get(component)? else {
            return Err(error(
                "Vérifiez les mises à jour avant d’installer.",
                "UPDATE_CHECK_REQUIRED",
            ));
        };
        let bytes = STANDARD.decode(&receipt.manifest_base64).map_err(|_| {
            error(
                "Reprise de mise à jour indisponible.",
                "UPDATE_STATE_UNAVAILABLE",
            )
        })?;
        let verified = self
            .verify_install(component, &bytes, &receipt.signature)
            .map_err(|_| {
                error(
                    "La mise à jour acceptée a expiré ou son identité a changé.",
                    "UPDATE_RECHECK_REQUIRED",
                )
            })?;
        Ok(AvailableRelease {
            verified,
            bytes,
            signature: receipt.signature,
        })
    }
    pub async fn install(
        self: &Arc<Self>,
        app: tauri::AppHandle,
        component: Component,
        confirmed: bool,
        rendering: Arc<AtomicBool>,
    ) -> Result<Value, UiError> {
        if !confirmed {
            return Err(error(
                "Confirmez Installer maintenant.",
                "UPDATE_CONFIRMATION_REQUIRED",
            ));
        }
        if rendering.load(Ordering::Acquire) {
            return Err(error(
                "Terminez la lecture avant d’installer.",
                "UPDATE_BUSY",
            ));
        }
        // Cross-process serialization also protects bounded cache filenames
        // when two installed/dev bundles happen to share the device settings.
        let _lease = InstallLease::acquire(&self.directory)?;
        if self.active.swap(true, Ordering::AcqRel) {
            return Err(error("Une mise à jour est déjà en cours.", "UPDATE_BUSY"));
        }
        let result = self.install_inner(app.clone(), component, rendering).await;
        self.active.store(false, Ordering::Release);
        result
    }
    /// Finalize only an already-installed exact slot. This never downloads or
    /// installs expired bytes. Expiry remains mandatory on every install path.
    fn finalize_installed_receipt(&self, component: Component) -> Result<Option<Version>, UiError> {
        if let Some(receipt) = PendingStore::open(&self.directory)?.get(component)? {
            let installed = Version::parse(&if component == Component::Mac {
                env!("CARGO_PKG_VERSION").into()
            } else {
                self.theme_version()
            })
            .map_err(|_| error("Version installée invalide.", "UPDATE_STATE_UNAVAILABLE"))?;
            let installed_source_matches = if component == Component::Mac {
                receipt.source_commit == env!("CHILL_SOURCE_COMMIT")
            } else {
                let current = self.directory.join("updates/themes/current");
                fs::read_link(&current).ok()
                    == Some(PathBuf::from(format!(
                        "releases/{}-{}",
                        receipt.version, receipt.sequence
                    )))
                    && chill_updates::validate_theme_package(&current).is_ok()
            };
            if installed == receipt.version && installed_source_matches {
                let bytes = STANDARD
                    .decode(&receipt.manifest_base64)
                    .map_err(|_| error("Reçu invalide.", "UPDATE_RECHECK_REQUIRED"))?;
                let verification_time =
                    time::OffsetDateTime::from_unix_timestamp(receipt.accepted_unix)
                        .map_err(|_| error("Reçu invalide.", "UPDATE_RECHECK_REQUIRED"))?;
                if verification_time > time::OffsetDateTime::now_utc() {
                    return Err(error("Reçu invalide.", "UPDATE_RECHECK_REQUIRED"));
                }
                let verified = self
                    .verify_at(component, &bytes, &receipt.signature, verification_time)
                    .map_err(|_| error("Reçu signé invalide.", "UPDATE_RECHECK_REQUIRED"))?;
                let state = self
                    .store()?
                    .read()
                    .map_err(|_| error("État indisponible.", "UPDATE_STATE_UNAVAILABLE"))?;
                let highest = state
                    .components
                    .get(&component)
                    .map_or(0, |entry| entry.highest_sequence);
                if highest > receipt.sequence {
                    return Err(error(
                        "Séquence installée incohérente.",
                        "UPDATE_RECHECK_REQUIRED",
                    ));
                }
                if highest < receipt.sequence {
                    self.store()?
                        .record_sequence(component, receipt.sequence)
                        .map_err(|_| error("Reçu non finalisé.", "UPDATE_STATE_UNAVAILABLE"))?;
                }
                PendingStore::open(&self.directory)?.commit(component, &verified, &bytes)?;
                return Ok(Some(receipt.version));
            }
        }
        Ok(None)
    }
    async fn install_inner(
        self: &Arc<Self>,
        app: tauri::AppHandle,
        component: Component,
        rendering: Arc<AtomicBool>,
    ) -> Result<Value, UiError> {
        if let Some(version) = self.finalize_installed_receipt(component)?
            && !self
                .available
                .lock()
                .map_err(|_| error("Mise à jour occupée.", "UPDATE_BUSY"))?
                .contains_key(&component)
        {
            return Ok(
                json!({"installed":true,"recovered":true,"component":component.as_str(),"version":version.to_string()}),
            );
        }
        let release = self.release_for_install(component)?;
        let verified = self
            .verify_install(component, &release.bytes, &release.signature)
            .map_err(|_| {
                error(
                    "La publication a expiré ou n’est plus compatible. Vérifiez à nouveau.",
                    "UPDATE_RECHECK_REQUIRED",
                )
            })?;
        let root = self.directory.join("updates/downloads");
        let asset = verified.selected_asset.clone();
        let download = tauri::async_runtime::spawn_blocking(move || {
            std::fs::create_dir_all(&root)?;
            // At most one bounded cache file per local component, including
            // after a crash. No UUID directories accumulate between attempts.
            let destination = root.join(format!("{}-download", component.as_str()));
            if destination.try_exists()? {
                let metadata = std::fs::symlink_metadata(&destination)?;
                if !metadata.file_type().is_file() {
                    return Err(chill_updates::UpdateError::Invalid("cache entry".into()));
                }
                std::fs::remove_file(&destination)?;
            }
            let guard = DownloadedAsset(destination.clone());
            GithubReleaseClient::new(TransportLimits {
                max_asset_bytes: 256 * 1024 * 1024,
                ..Default::default()
            })?
            .download_verified(
                &asset.url,
                &destination,
                asset.byte_length,
                &asset.sha256,
            )?;
            Ok::<_, chill_updates::UpdateError>(guard)
        })
        .await
        .map_err(|_| error("Téléchargement interrompu.", "UPDATE_DOWNLOAD_FAILED"))?
        .map_err(|_| {
            error(
                "Téléchargement ou signature invalide.",
                "UPDATE_DOWNLOAD_FAILED",
            )
        })?;
        if rendering.load(Ordering::Acquire) {
            return Err(error(
                "Installation différée : une lecture a commencé.",
                "UPDATE_BUSY",
            ));
        }
        if component == Component::Mac {
            let signature = verified
                .selected_asset
                .tauri_signature
                .as_ref()
                .ok_or_else(|| error("Signature Mac manquante.", "UPDATE_SIGNATURE_INVALID"))?;
            let updater = app
                .updater_builder()
                .endpoints(vec![
                    "https://api.github.com/repos/ungeekrange-cmyk/chillcenter/releases"
                        .parse()
                        .unwrap(),
                ])
                .map_err(|_| error("Mise à jour indisponible.", "UPDATE_FAILED"))?
                .build()
                .map_err(|_| error("Installateur indisponible.", "UPDATE_FAILED"))?;
            let raw = json!({"version":verified.manifest.version.to_string(),"platforms":{"darwin-aarch64":{"url":verified.selected_asset.url,"signature":signature}}});
            let update = updater
                .restore_update(raw)
                .map_err(|_| error("Métadonnées Mac invalides.", "UPDATE_SIGNATURE_INVALID"))?
                .ok_or_else(|| error("Aucune version plus récente.", "UPDATE_NOT_NEWER"))?;
            confirm_macos_install(&app, &verified.manifest.version, rendering.as_ref()).await?;
            let verified = self
                .verify_install(component, &release.bytes, &release.signature)
                .map_err(|_| {
                    error(
                        "Publication expirée ou incompatible : aucune installation.",
                        "UPDATE_RECHECK_REQUIRED",
                    )
                })?;
            // Persist the exact signed receipt before any executable mutation.
            // A failed/interrupted install can only resume these same bytes.
            PendingStore::open(&self.directory)?.accept(
                component,
                &verified,
                &release.bytes,
                &release.signature,
            )?;
            tauri::async_runtime::spawn_blocking(move || {
                let bytes = std::fs::read(&download.0)
                    .map_err(|_| error("Paquet indisponible.", "UPDATE_DOWNLOAD_FAILED"))?;
                update.install(bytes).map_err(|_| {
                    error(
                        "Installation impossible. L’application actuelle est conservée.",
                        "UPDATE_INSTALL_FAILED",
                    )
                })
            })
            .await
            .map_err(|_| error("Installation interrompue.", "UPDATE_INSTALL_FAILED"))??;
            self.store()?
                .record_sequence(component, verified.manifest.release_sequence)
                .map_err(|_| {
                    error(
                        "Installation terminée mais le reçu ne peut pas être finalisé.",
                        "UPDATE_STATE_UNAVAILABLE",
                    )
                })?;
            PendingStore::open(&self.directory)?.commit(component, &verified, &release.bytes)?;
            app.restart();
        }
        if component != Component::Themes {
            return Err(error("Composant invalide.", "UPDATE_COMPONENT_INVALID"));
        }
        let host = self.clone();
        let version = verified.manifest.version.to_string();
        tauri::async_runtime::spawn_blocking(move || {
            let verified = host
                .verify_install(component, &release.bytes, &release.signature)
                .map_err(|_| {
                    error(
                        "Publication expirée ou incompatible : aucun pack installé.",
                        "UPDATE_RECHECK_REQUIRED",
                    )
                })?;
            PendingStore::open(&host.directory)?.accept(
                component,
                &verified,
                &release.bytes,
                &release.signature,
            )?;
            let layout = chill_updates::InstallLayout {
                component_root: host.directory.join("updates/themes"),
                backup_root: host.directory.join("updates/theme-backups"),
            };
            for path in [&layout.component_root, &layout.backup_root] {
                std::fs::create_dir_all(path).map_err(|_| {
                    error(
                        "Stockage des ambiances indisponible.",
                        "THEME_UPDATE_FAILED",
                    )
                })?;
            }
            let limits = chill_updates::ExtractionLimits::default();
            let request = chill_updates::ApplyRequest {
                operation_id: uuid::Uuid::new_v4(),
                verified: &verified,
                artifact: &download.0,
                layout: &layout,
                limits: &limits,
                allow_first_install: true,
                force_interrupt: false,
            };
            let mut hooks = ThemeHooks { rendering };
            chill_updates::apply_staged(&request, &mut hooks).map_err(|_| {
                error(
                    "Pack de thèmes refusé. L’ancien pack reste disponible.",
                    "THEME_UPDATE_FAILED",
                )
            })?;
            host.store()?
                .record_sequence(component, verified.manifest.release_sequence)
                .map_err(|_| {
                    error(
                        "Pack installé mais reçu non finalisé.",
                        "UPDATE_STATE_UNAVAILABLE",
                    )
                })?;
            PendingStore::open(&host.directory)?.commit(component, &verified, &release.bytes)?;
            Ok::<_, UiError>(())
        })
        .await
        .map_err(|_| error("Installation de thèmes interrompue.", "THEME_UPDATE_FAILED"))??;
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.eval("window.reloadThemePack?.()");
        }
        Ok(json!({"installed":true,"component":"themes","version":version}))
    }
    pub fn start_schedule(self: &Arc<Self>, app: tauri::AppHandle, rendering: Arc<AtomicBool>) {
        let host = self.clone();
        tauri::async_runtime::spawn(async move {
            let mut timer = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                timer.tick().await;
                let copy = host.clone();
                let due = tauri::async_runtime::spawn_blocking(move || {
                    copy.store()?
                        .update_scheduler(|state| {
                            chill_updates::due_attempts_for(
                                state,
                                time::OffsetDateTime::now_utc(),
                                local_offset(),
                                &[Component::Mac, Component::Themes],
                            )
                        })
                        .map_err(|_| {
                            error("Planification indisponible.", "UPDATE_STATE_UNAVAILABLE")
                        })
                })
                .await;
                if let Ok(Ok(due)) = due {
                    for attempt in due {
                        if !matches!(attempt.component, Component::Mac | Component::Themes) {
                            continue;
                        }
                        for _ in 0..600 {
                            if !host.active.load(Ordering::Acquire) {
                                break;
                            }
                            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        }
                        let _ = host.check(attempt.component);
                        for _ in 0..600 {
                            if !host.active.load(Ordering::Acquire) {
                                break;
                            }
                            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        }
                        if attempt.component == Component::Themes {
                            host.pending_theme.store(true, Ordering::Release);
                        }
                    }
                }
                // A busy player defers the same checked release until idle;
                // it does not lose the scheduled attempt for the entire day.
                if host.pending_theme.load(Ordering::Acquire)
                    && !rendering.load(Ordering::Acquire)
                    && !host.active.load(Ordering::Acquire)
                {
                    let automatic = host
                        .store()
                        .and_then(|s| {
                            s.read().map_err(|_| {
                                error("Réglages indisponibles.", "UPDATE_STATE_UNAVAILABLE")
                            })
                        })
                        .is_ok_and(|s| {
                            s.policies
                                .get(&Component::Themes)
                                .is_some_and(|p| p.mode == PolicyMode::Automatic)
                        });
                    let available = host
                        .available
                        .lock()
                        .is_ok_and(|a| a.contains_key(&Component::Themes));
                    host.pending_theme.store(false, Ordering::Release);
                    if automatic && available {
                        let result = host
                            .install(app.clone(), Component::Themes, true, rendering.clone())
                            .await;
                        if result.is_err_and(|error| error.code == "UPDATE_BUSY") {
                            host.pending_theme.store(true, Ordering::Release);
                        }
                    }
                }
            }
        });
    }
}
struct ThemeHooks {
    rendering: Arc<AtomicBool>,
}
impl chill_updates::InstallHooks for ThemeHooks {
    fn is_idle(&mut self, _: Component) -> chill_updates::Result<bool> {
        Ok(!self.rendering.load(Ordering::Acquire))
    }
    fn begin_quiescence(&mut self, _: Component, _: bool) -> chill_updates::Result<()> {
        Ok(())
    }
    fn backup(&mut self, _: Component, _: &std::path::Path) -> chill_updates::Result<String> {
        Ok(format!("themes-{}", uuid::Uuid::new_v4()))
    }
    fn activate(&mut self, _: Component, _: &std::path::Path) -> chill_updates::Result<()> {
        Ok(())
    }
    fn health_check(&mut self, _: Component, path: &std::path::Path) -> chill_updates::Result<()> {
        if !crate::theme_bridge::validate_directory(path) {
            return Err(chill_updates::UpdateError::Incompatible(
                "theme renderer".into(),
            ));
        }
        Ok(())
    }
    fn end_quiescence(&mut self, _: Component) -> chill_updates::Result<()> {
        Ok(())
    }
}
pub fn plugin() -> tauri::plugin::TauriPlugin<tauri::Wry, tauri_plugin_updater::Config> {
    tauri_plugin_updater::Builder::new()
        .pubkey(STANDARD.encode(PUBLIC_KEY))
        .build()
}
fn local_offset() -> time::UtcOffset {
    let now = unsafe { libc::time(std::ptr::null_mut()) };
    let mut out = std::mem::MaybeUninit::<libc::tm>::uninit();
    if unsafe { libc::localtime_r(&now, out.as_mut_ptr()) }.is_null() {
        return time::UtcOffset::UTC;
    }
    time::UtcOffset::from_whole_seconds(unsafe { out.assume_init() }.tm_gmtoff as i32)
        .unwrap_or(time::UtcOffset::UTC)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_updater_configuration_deserializes_before_builder_overrides() {
        // Plugin initialization deserializes config BEFORE Builder::pubkey is
        // applied. A missing pubkey prevented the whole native app from opening.
        let config: Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let config: tauri_plugin_updater::Config =
            serde_json::from_value(config["plugins"]["updater"].clone()).unwrap();
        assert_eq!(
            STANDARD.decode(config.pubkey).unwrap(),
            PUBLIC_KEY.as_bytes()
        );
        assert!(config.require_signed_version);
        assert!(!config.dangerous_insecure_transport_protocol);
        assert!(!config.dangerous_accept_invalid_certs);
        assert!(!config.dangerous_accept_invalid_hostnames);
        assert!(!config.allow_downgrades);
        assert!(config.endpoints.is_empty());
    }

    fn receipt(bytes: &[u8]) -> PendingRelease {
        PendingRelease {
            sequence: 7,
            version: Version::parse("0.7.1").unwrap(),
            source_commit: "a".repeat(40),
            accepted_unix: 1_792_396_800,
            manifest_sha256: hex_digest(bytes),
            manifest_base64: STANDARD.encode(bytes),
            signature: "synthetic-signature".into(),
        }
    }
    #[test]
    fn only_one_instance_can_install_and_lock_is_released() {
        let directory = tempfile::tempdir().unwrap();
        let first = InstallLease::acquire(directory.path()).unwrap();
        assert!(InstallLease::acquire(directory.path()).is_err());
        drop(first);
        assert!(InstallLease::acquire(directory.path()).is_ok());
    }
    #[test]
    fn abandoned_download_is_removed_without_recursion() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("mac-download");
        std::fs::write(&path, b"fixture").unwrap();
        drop(DownloadedAsset(path.clone()));
        assert!(!path.exists());
    }
    #[test]
    fn pending_receipt_survives_restart_only_for_exact_cached_manifest() {
        let directory = tempfile::tempdir().unwrap();
        let store = PendingStore::open(directory.path()).unwrap();
        let original = b"signed manifest bytes";
        let receipt = receipt(original);
        store
            .save(&PendingState {
                schema_version: PENDING_SCHEMA_VERSION,
                releases: BTreeMap::from([(Component::Mac, receipt.clone())]),
            })
            .unwrap();
        assert_eq!(
            PendingStore::open(directory.path())
                .unwrap()
                .get(Component::Mac)
                .unwrap(),
            Some(receipt)
        );
        let path = directory.path().join("updates/local-pending/pending.json");
        let mut changed: PendingState =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        changed
            .releases
            .get_mut(&Component::Mac)
            .unwrap()
            .manifest_base64 = STANDARD.encode(b"substituted bytes");
        std::fs::write(&path, serde_json::to_vec(&changed).unwrap()).unwrap();
        assert!(
            PendingStore::open(directory.path())
                .unwrap()
                .get(Component::Mac)
                .is_err()
        );
    }
    #[test]
    fn pending_store_rejects_unavailable_or_symlinked_disk_state() {
        let file = tempfile::NamedTempFile::new().unwrap();
        assert!(PendingStore::open(file.path()).is_err());
        let directory = tempfile::tempdir().unwrap();
        std::fs::create_dir(directory.path().join("updates")).unwrap();
        std::os::unix::fs::symlink("/tmp", directory.path().join("updates/local-pending")).unwrap();
        assert!(PendingStore::open(directory.path()).is_err());
    }
    #[test]
    fn pending_receipt_refuses_same_sequence_substitution_but_allows_newer_recovery() {
        let directory = tempfile::tempdir().unwrap();
        let store = PendingStore::open(directory.path()).unwrap();
        let first = receipt(b"first signed manifest");
        store
            .save(&PendingState {
                schema_version: PENDING_SCHEMA_VERSION,
                releases: BTreeMap::from([(Component::Mac, first.clone())]),
            })
            .unwrap();
        let mut changed = first.clone();
        changed.manifest_base64 = STANDARD.encode(b"changed bytes");
        changed.manifest_sha256 = hex_digest(b"changed bytes");
        assert!(
            store
                .with_lock(|state| {
                    let existing = state.releases.get(&Component::Mac).unwrap();
                    if existing.sequence == changed.sequence && existing != &changed {
                        return Err(error("substitution", "UPDATE_RECOVERY_REQUIRED"));
                    }
                    Ok(((), false))
                })
                .is_err()
        );
        let mut newer = changed;
        newer.sequence += 1;
        store
            .with_lock(|state| {
                state.releases.insert(Component::Mac, newer.clone());
                Ok(((), true))
            })
            .unwrap();
        assert_eq!(
            store.get(Component::Mac).unwrap().unwrap().sequence,
            first.sequence + 1
        );
    }
    #[test]
    fn renderer_cannot_supply_consent_url_path_or_force() {
        for value in [
            json!({"action":"install","component":"mac","confirmed":true}),
            json!({"action":"install","component":"mac","url":"https://invalid.example"}),
            json!({"action":"install","component":"themes","force":true}),
        ] {
            assert!(serde_json::from_value::<UpdateCommand>(value).is_err());
        }
        assert!(
            serde_json::from_value::<UpdateCommand>(json!({"action":"install","component":"mac"}))
                .is_ok()
        );
    }

    #[test]
    fn minisign_pending_receipt_allows_only_the_exact_pre_expiry_manifest() {
        use std::process::Command;
        use time::format_description::well_known::Rfc3339;

        let directory = tempfile::tempdir().unwrap();
        let key_directory = tempfile::tempdir().unwrap();
        let secret_key = key_directory.path().join("fixture.key");
        let public_key = key_directory.path().join("fixture.pub");
        assert!(
            Command::new("/opt/homebrew/bin/minisign")
                .args([
                    "-G",
                    "-W",
                    "-s",
                    secret_key.to_str().unwrap(),
                    "-p",
                    public_key.to_str().unwrap(),
                ])
                .status()
                .unwrap()
                .success()
        );
        let public_key_text = fs::read_to_string(&public_key).unwrap();
        let key_id = public_key_text
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().last())
            .unwrap()
            .to_owned();
        let sign = |name: &str, bytes: &[u8]| {
            let manifest = key_directory.path().join(format!("{name}.json"));
            let signature = key_directory.path().join(format!("{name}.minisig"));
            fs::write(&manifest, bytes).unwrap();
            assert!(
                Command::new("/opt/homebrew/bin/minisign")
                    .args([
                        "-S",
                        "-s",
                        secret_key.to_str().unwrap(),
                        "-m",
                        manifest.to_str().unwrap(),
                        "-x",
                        signature.to_str().unwrap(),
                    ])
                    .status()
                    .unwrap()
                    .success()
            );
            fs::read_to_string(signature).unwrap()
        };
        let manifest = |source_commit: &str, expires_at: &str| {
            serde_json::to_vec(&json!({
                "schema_version": 1,
                "component": "mac",
                "version": "0.7.1",
                "release_sequence": 7,
                "release_tag": "mac-v0.7.1",
                "source_commit": source_commit,
                "issued_at": "2026-10-18T00:00:00Z",
                "expires_at": expires_at,
                "key_id": key_id,
                "compatibility": {
                    "os_arch": ["darwin-aarch64"],
                    "api": {"min": "2026.10.14", "max": "2026.10.14"}
                },
                "assets": [{
                    "target": "darwin-aarch64",
                    "url": "https://github.com/ungeekrange-cmyk/chillcenter/releases/download/mac-v0.7.1/test-mac",
                    "byte_length": 7,
                    "sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                    "format": "raw",
                    "tauri_signature": "test-only-tauri-signature"
                }]
            }))
            .unwrap()
        };
        let exact_bytes = manifest(&"a".repeat(40), "2026-10-19T00:00:00Z");
        let exact_signature = sign("exact", &exact_bytes);
        let accepted = time::OffsetDateTime::parse("2026-10-18T12:00:00Z", &Rfc3339).unwrap();
        let after_expiry = time::OffsetDateTime::parse("2026-10-20T00:00:00Z", &Rfc3339).unwrap();
        let host = UpdateHost::new(directory.path().to_path_buf());

        // The original, real Minisign signature was valid at the receipt time.
        let accepted_verified = host
            .verify_at_using_key(
                Component::Mac,
                &exact_bytes,
                &exact_signature,
                accepted,
                &public_key_text,
                &key_id,
            )
            .unwrap();
        // A new discovery must never accept that expired publication.
        assert!(
            host.verify_at_using_key(
                Component::Mac,
                &exact_bytes,
                &exact_signature,
                after_expiry,
                &public_key_text,
                &key_id,
            )
            .is_err()
        );

        let store = PendingStore::open(directory.path()).unwrap();
        store
            .save(&PendingState {
                schema_version: PENDING_SCHEMA_VERSION,
                releases: BTreeMap::from([(
                    Component::Mac,
                    PendingRelease {
                        sequence: accepted_verified.manifest.release_sequence,
                        version: accepted_verified.manifest.version.clone(),
                        source_commit: accepted_verified.manifest.source_commit.clone(),
                        accepted_unix: accepted.unix_timestamp(),
                        manifest_sha256: hex_digest(&exact_bytes),
                        manifest_base64: STANDARD.encode(&exact_bytes),
                        signature: exact_signature.clone(),
                    },
                )]),
            })
            .unwrap();
        // Expiry is intentionally evaluated at acceptance only for this exact
        // persisted signed byte sequence.
        assert!(
            host.verify_install_using_key(
                Component::Mac,
                &exact_bytes,
                &exact_signature,
                &public_key_text,
                &key_id,
            )
            .is_ok()
        );

        let substituted_bytes = manifest(&"c".repeat(40), "2026-10-21T00:00:00Z");
        let substituted_signature = sign("substituted", &substituted_bytes);
        // A separately valid signature cannot substitute a different manifest
        // with the same sequence while an exact receipt remains pending.
        assert!(
            host.verify_install_using_key(
                Component::Mac,
                &substituted_bytes,
                &substituted_signature,
                &public_key_text,
                &key_id,
            )
            .is_err()
        );

        store
            .with_lock(|state| {
                state
                    .releases
                    .get_mut(&Component::Mac)
                    .unwrap()
                    .accepted_unix = after_expiry.unix_timestamp() + 1;
                Ok(((), true))
            })
            .unwrap();
        // A forged/future acceptance timestamp is never a recovery bypass.
        assert!(
            host.verify_install_using_key(
                Component::Mac,
                &exact_bytes,
                &exact_signature,
                &public_key_text,
                &key_id,
            )
            .is_err()
        );
    }

    #[test]
    fn macos_post_consent_guard_keeps_the_existing_busy_code() {
        let rendering = AtomicBool::new(true);
        let error = macos_post_consent_rendering_guard(&rendering).unwrap_err();
        assert_eq!(error.code, "UPDATE_BUSY");
        assert_eq!(
            error.message,
            "Une lecture a commencé ; relancez l’installation après sa fin."
        );
        rendering.store(false, Ordering::Release);
        assert!(macos_post_consent_rendering_guard(&rendering).is_ok());
    }
}
