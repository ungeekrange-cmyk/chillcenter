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
use std::{
    collections::BTreeMap,
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
        let installed = Version::parse(&if component == Component::Mac {
            env!("CARGO_PKG_VERSION").into()
        } else {
            self.theme_version()
        })
        .map_err(|_| chill_updates::UpdateError::Invalid("installed version".into()))?;
        let highest = StateStore::open(self.directory.join("updates/local-state"))?
            .read()?
            .components
            .get(&component)
            .map_or(0, |s| s.highest_sequence);
        let context = VerificationContext {
            now: time::OffsetDateTime::now_utc(),
            expected_key_id: KEY_ID,
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
        chill_updates::parse_and_verify_manifest(bytes, signature, PUBLIC_KEY, &context)
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
    async fn install_inner(
        self: &Arc<Self>,
        app: tauri::AppHandle,
        component: Component,
        rendering: Arc<AtomicBool>,
    ) -> Result<Value, UiError> {
        let release = self
            .available
            .lock()
            .map_err(|_| error("Mise à jour occupée.", "UPDATE_BUSY"))?
            .get(&component)
            .cloned()
            .ok_or_else(|| {
                error(
                    "Vérifiez les mises à jour avant d’installer.",
                    "UPDATE_CHECK_REQUIRED",
                )
            })?;
        let verified = self
            .verify_current(component, &release.bytes, &release.signature)
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
            let (answer, receive) = tokio::sync::oneshot::channel();
            app.dialog().message(format!("Installer Chill Center {} et redémarrer maintenant ?\nInstall Chill Center {} and restart now?",verified.manifest.version,verified.manifest.version))
                .title("Chill Center").kind(MessageDialogKind::Warning)
                .buttons(MessageDialogButtons::OkCancelCustom("Installer maintenant / Install now".into(),"Annuler / Cancel".into()))
                .show(move |confirmed|{let _=answer.send(confirmed);});
            if !receive.await.unwrap_or(false) {
                return Err(error("Installation annulée.", "UPDATE_CANCELLED"));
            }
            if rendering.load(Ordering::Acquire) {
                return Err(error(
                    "Une lecture a commencé ; relancez l’installation après sa fin.",
                    "UPDATE_BUSY",
                ));
            }
            let verified = self
                .verify_current(component, &release.bytes, &release.signature)
                .map_err(|_| {
                    error(
                        "Publication expirée ou incompatible : aucune installation.",
                        "UPDATE_RECHECK_REQUIRED",
                    )
                })?;
            // Acceptance is durable before switching any executable bytes. A
            // failed install keeps the sequence: recovery is local/explicit,
            // never an online downgrade or an implicit verification bypass.
            self.store()?
                .record_sequence(component, verified.manifest.release_sequence)
                .map_err(|_| {
                    error(
                        "Acceptation non enregistrée : aucune installation.",
                        "UPDATE_STATE_UNAVAILABLE",
                    )
                })?;
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
            app.restart();
        }
        if component != Component::Themes {
            return Err(error("Composant invalide.", "UPDATE_COMPONENT_INVALID"));
        }
        let host = self.clone();
        let version = verified.manifest.version.to_string();
        tauri::async_runtime::spawn_blocking(move || {
            let verified = host
                .verify_current(component, &release.bytes, &release.signature)
                .map_err(|_| {
                    error(
                        "Publication expirée ou incompatible : aucun pack installé.",
                        "UPDATE_RECHECK_REQUIRED",
                    )
                })?;
            host.store()?
                .record_sequence(component, verified.manifest.release_sequence)
                .map_err(|_| {
                    error(
                        "Acceptation non enregistrée : aucun pack installé.",
                        "UPDATE_STATE_UNAVAILABLE",
                    )
                })?;
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
                            chill_updates::due_attempts(
                                state,
                                time::OffsetDateTime::now_utc(),
                                local_offset(),
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
}
