//! Native administration owns chooser paths, streamed files and bound server jobs.
use chill_client::{
    HttpMethod, Transport,
    ui::{PlaybackConnection, UiError},
};
use chill_domain::import::ImportMapping;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
pub const MAX_ARCHIVE: u64 = 1024 * 1024 * 1024;
pub fn error(message: &str, code: &str) -> UiError {
    UiError {
        message: message.into(),
        code: code.into(),
        status: None,
    }
}
#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum AdminCommand {
    Import {},
    Export {},
    Status { id: String },
    Cancel { id: String },
    Save { id: String },
    Plan { id: String, mapping: ImportMapping },
    Apply { id: String, confirmed: bool },
}
#[derive(Clone, Serialize)]
pub struct View {
    pub id: String,
    pub kind: &'static str,
    pub stage: &'static str,
    pub result: Option<Value>,
    pub error: Option<String>,
    pub error_code: Option<String>,
}
pub struct Job {
    pub connection: PlaybackConnection,
    pub cancelled: Arc<AtomicBool>,
    pub view: View,
    pub preview: Option<String>,
    pub export: Option<String>,
    pub plan: Option<String>,
    pub busy: bool,
    pub created: Instant,
}
#[derive(Default)]
pub struct AdminHost {
    pub jobs: Mutex<HashMap<String, Arc<Mutex<Job>>>>,
}
impl AdminHost {
    pub fn begin(
        &self,
        connection: PlaybackConnection,
        kind: &'static str,
    ) -> Result<Arc<Mutex<Job>>, UiError> {
        let mut jobs = self
            .jobs
            .lock()
            .map_err(|_| error("Administration indisponible.", "ADMIN_BUSY"))?;
        jobs.retain(|_, job| {
            job.lock().is_ok_and(|j| {
                j.busy
                    || (!["cancelled", "saved", "applied"].contains(&j.view.stage)
                        && j.created.elapsed() < Duration::from_secs(900))
            })
        });
        if jobs.len() >= 6 || jobs.values().any(|job| job.lock().is_ok_and(|j| j.busy)) {
            return Err(error(
                "Une opération d’archive est déjà en cours.",
                "ADMIN_BUSY",
            ));
        }
        let id = uuid::Uuid::new_v4().to_string();
        let job = Arc::new(Mutex::new(Job {
            connection,
            cancelled: Arc::new(AtomicBool::new(false)),
            view: View {
                id: id.clone(),
                kind,
                stage: "working",
                result: None,
                error: None,
                error_code: None,
            },
            preview: None,
            export: None,
            plan: None,
            busy: true,
            created: Instant::now(),
        }));
        jobs.insert(id, job.clone());
        Ok(job)
    }
    pub fn get(&self, id: &str) -> Result<Arc<Mutex<Job>>, UiError> {
        uuid::Uuid::parse_str(id).map_err(|_| error("Opération invalide.", "ADMIN_INVALID"))?;
        self.jobs
            .lock()
            .map_err(|_| error("Administration indisponible.", "ADMIN_BUSY"))?
            .get(id)
            .cloned()
            .ok_or_else(|| error("Opération expirée.", "ADMIN_NOT_FOUND"))
    }
    pub fn cancel_all(&self) {
        if let Ok(jobs) = self.jobs.lock() {
            for job in jobs.values() {
                cancel(job.clone());
            }
        }
    }
}
fn owned_id(value: &Value, key: &str) -> Result<String, UiError> {
    let id = value[key]
        .as_str()
        .ok_or_else(|| error("Réponse archive invalide.", "ADMIN_INVALID"))?;
    uuid::Uuid::parse_str(id).map_err(|_| error("Réponse archive invalide.", "ADMIN_INVALID"))?;
    Ok(id.into())
}
pub fn check(job: &Job, current: &PlaybackConnection) -> Result<(), UiError> {
    if job.cancelled.load(Ordering::Acquire)
        || job.connection.server_id != current.server_id
        || job.connection.profile_id != current.profile_id
    {
        return Err(error(
            "Le serveur ou le profil a changé. Recommencez l’opération.",
            "ADMIN_STALE",
        ));
    }
    Ok(())
}
pub fn selected_zip(path: &Path) -> Result<File, UiError> {
    if !path.is_absolute() {
        return Err(error("Archive invalide.", "ARCHIVE_INVALID"));
    }
    let before = fs::symlink_metadata(path)
        .map_err(|_| error("Archive inaccessible.", "ARCHIVE_UNAVAILABLE"))?;
    if !before.is_file() || before.file_type().is_symlink() || before.len() > MAX_ARCHIVE {
        return Err(error(
            "Archive ZIP invalide ou trop volumineuse (1 Gio).",
            "ARCHIVE_INVALID",
        ));
    }
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| error("Archive inaccessible.", "ARCHIVE_UNAVAILABLE"))?;
    let after = file
        .metadata()
        .map_err(|_| error("Archive inaccessible.", "ARCHIVE_UNAVAILABLE"))?;
    if version(&before) != version(&after) {
        return Err(error(
            "Archive modifiée pendant la sélection.",
            "ARCHIVE_CHANGED",
        ));
    }
    Ok(file)
}
fn version(m: &fs::Metadata) -> (u64, u64, u64, i64, i64, i64, i64) {
    (
        m.dev(),
        m.ino(),
        m.len(),
        m.mtime(),
        m.mtime_nsec(),
        m.ctime(),
        m.ctime_nsec(),
    )
}
pub struct Output {
    partial: PathBuf,
    target: PathBuf,
    published: bool,
}
impl Drop for Output {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.partial);
    }
}
impl Output {
    pub fn create(path: &Path) -> Result<(Self, File), UiError> {
        if !path.is_absolute()
            || path.extension().and_then(|e| e.to_str()) != Some("zip")
            || fs::symlink_metadata(path).is_ok()
        {
            return Err(error(
                "Choisissez un nouveau fichier ZIP, sans écraser de fichier.",
                "ARCHIVE_DESTINATION",
            ));
        }
        let parent = path
            .parent()
            .ok_or_else(|| error("Destination invalide.", "ARCHIVE_DESTINATION"))?;
        let partial = parent.join(format!(".chillcenter-{}.partial", uuid::Uuid::new_v4()));
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&partial)
            .map_err(|_| error("Destination inaccessible.", "ARCHIVE_DESTINATION"))?;
        Ok((
            Self {
                partial,
                target: path.into(),
                published: false,
            },
            file,
        ))
    }
    pub fn publish(mut self) -> Result<(), UiError> {
        // Atomic no-replace, including dangling destination symlinks.
        fs::hard_link(&self.partial, &self.target).map_err(|_| {
            error(
                "Destination déjà existante ou inaccessible.",
                "ARCHIVE_DESTINATION",
            )
        })?;
        self.published = true;
        let _ = fs::remove_file(&self.partial);
        File::open(self.target.parent().unwrap())
            .and_then(|d| d.sync_all())
            .map_err(|_| {
                error(
                    "Export créé mais synchronisation disque incertaine. Vérifiez le fichier.",
                    "ARCHIVE_SYNC",
                )
            })?;
        Ok(())
    }
}
async fn cancellable<T>(
    flag: &AtomicBool,
    future: impl std::future::Future<Output = Result<T, UiError>>,
) -> Result<T, UiError> {
    tokio::select! {biased;_ =async{while !flag.load(Ordering::Acquire){tokio::time::sleep(Duration::from_millis(50)).await;}}=>Err(error("Opération annulée.","ADMIN_CANCELLED")),result=future=>result}
}
fn handles(job: &Arc<Mutex<Job>>) -> Result<(Transport, Arc<AtomicBool>), UiError> {
    let job = job
        .lock()
        .map_err(|_| error("Administration indisponible.", "ADMIN_BUSY"))?;
    Ok((job.connection.transport.clone(), job.cancelled.clone()))
}
fn finish(job: &Arc<Mutex<Job>>, result: Result<Value, UiError>, stage: &'static str) {
    if let Ok(mut job) = job.lock() {
        job.busy = false;
        if job.cancelled.load(Ordering::Acquire) && stage != "applied" {
            job.view.stage = "cancelled";
            job.view.result = None;
            return;
        }
        match result {
            Ok(value) => {
                job.view.stage = stage;
                job.view.result = Some(value);
                job.view.error = None;
                job.view.error_code = None;
            }
            Err(error) => {
                job.view.stage = "failed";
                job.view.error = Some(error.message);
                job.view.error_code = Some(error.code);
            }
        }
    }
}
pub async fn upload(job: Arc<Mutex<Job>>, file: File) {
    let result = async {
        let (transport, flag) = handles(&job)?;
        let before = file
            .metadata()
            .map_err(|_| error("Archive inaccessible.", "ARCHIVE_UNAVAILABLE"))?;
        let reader = file
            .try_clone()
            .map_err(|_| error("Archive inaccessible.", "ARCHIVE_UNAVAILABLE"))?;
        let value = cancellable(&flag, async {
            transport
                .upload_archive_file(reader, before.len())
                .await
                .map_err(Into::into)
        })
        .await?;
        let preview = owned_id(&value, "preview_id")?;
        {
            job.lock()
                .map_err(|_| error("Administration indisponible.", "ADMIN_BUSY"))?
                .preview = Some(preview.clone());
        }
        let after = file
            .metadata()
            .map_err(|_| error("Archive inaccessible.", "ARCHIVE_UNAVAILABLE"))?;
        if version(&before) != version(&after) || flag.load(Ordering::Acquire) {
            cleanup(transport, Some(preview), None).await;
            return Err(error(
                "Archive modifiée ou annulée. Aucun import appliqué.",
                "ARCHIVE_CHANGED",
            ));
        }
        Ok(value)
    }
    .await;
    finish(&job, result, "preview");
}
pub async fn export(job: Arc<Mutex<Job>>) {
    let result = async {
        let (transport, flag) = handles(&job)?;
        let started: Value = cancellable(&flag, async {
            transport
                .request_json::<(), Value>(HttpMethod::POST, "/api/2026-10-14/admin/exports", None)
                .await
                .map_err(Into::into)
        })
        .await?;
        let id = owned_id(&started, "export_id")?;
        {
            job.lock()
                .map_err(|_| error("Administration indisponible.", "ADMIN_BUSY"))?
                .export = Some(id.clone());
        }
        if flag.load(Ordering::Acquire) {
            cleanup(transport, None, Some(id)).await;
            return Err(error("Export annulé.", "ADMIN_CANCELLED"));
        }
        let deadline = Instant::now() + Duration::from_secs(600);
        loop {
            if Instant::now() > deadline {
                return Err(error(
                    "Export trop long. Annulez puis réessayez.",
                    "ADMIN_TIMEOUT",
                ));
            }
            let status: Value = cancellable(&flag, async {
                transport
                    .get_json(&format!("/api/2026-10-14/admin/exports/{id}/status"))
                    .await
                    .map_err(Into::into)
            })
            .await?;
            match status["state"].as_str() {
                Some("ready") => return Ok(status),
                Some("building") => tokio::time::sleep(Duration::from_millis(500)).await,
                _ => {
                    return Err(error(
                        "Export impossible : quota, limite ou stockage. Données conservées.",
                        "ARCHIVE_EXPORT_FAILED",
                    ));
                }
            }
        }
    }
    .await;
    finish(&job, result, "ready");
}
pub async fn save(job: Arc<Mutex<Job>>, output: Output, file: File) {
    let result = async {
        let (transport, flag) = handles(&job)?;
        let id = job
            .lock()
            .map_err(|_| error("Administration indisponible.", "ADMIN_BUSY"))?
            .export
            .clone()
            .ok_or_else(|| error("Export non prêt.", "ADMIN_INVALID"))?;
        let size = cancellable(&flag, async {
            transport
                .download_archive_file(&id, file)
                .await
                .map_err(Into::into)
        })
        .await?;
        if flag.load(Ordering::Acquire) {
            return Err(error("Export annulé.", "ADMIN_CANCELLED"));
        }
        tokio::task::spawn_blocking(move || output.publish())
            .await
            .map_err(|_| error("Enregistrement interrompu.", "ARCHIVE_WRITE"))??;
        Ok(json!({"saved":true,"bytes":size}))
    }
    .await;
    finish(&job, result, "saved");
}
pub async fn plan(job: Arc<Mutex<Job>>, mapping: ImportMapping) {
    let result = async {
        let (transport, flag) = handles(&job)?;
        let id = job
            .lock()
            .map_err(|_| error("Administration indisponible.", "ADMIN_BUSY"))?
            .preview
            .clone()
            .ok_or_else(|| error("Aperçu non prêt.", "ADMIN_INVALID"))?;
        let value: Value = cancellable(&flag, async {
            transport
                .request_json(
                    HttpMethod::POST,
                    &format!("/api/2026-10-14/admin/imports/{id}/plan"),
                    Some(&mapping),
                )
                .await
                .map_err(Into::into)
        })
        .await?;
        let plan = owned_id(&value, "plan_id")?;
        job.lock()
            .map_err(|_| error("Administration indisponible.", "ADMIN_BUSY"))?
            .plan = Some(plan);
        Ok(value)
    }
    .await;
    finish(&job, result, "planned");
}
pub async fn apply(job: Arc<Mutex<Job>>) {
    // Confirmed transactions may commit before an HTTP cancellation. Never
    // promise rollback from cancelling the response; receipt permits retries.
    let result = async {
        let (transport, flag) = handles(&job)?;
        if flag.load(Ordering::Acquire) {
            return Err(error("Import annulé.", "ADMIN_CANCELLED"));
        }
        let (id, plan) = {
            let state = job
                .lock()
                .map_err(|_| error("Administration indisponible.", "ADMIN_BUSY"))?;
            (
                state
                    .preview
                    .clone()
                    .ok_or_else(|| error("Aperçu absent.", "ADMIN_INVALID"))?,
                state
                    .plan
                    .clone()
                    .ok_or_else(|| error("Plan absent.", "ADMIN_INVALID"))?,
            )
        };
        transport
            .request_json(
                HttpMethod::POST,
                &format!("/api/2026-10-14/admin/imports/{id}/apply"),
                Some(&json!({"plan_id":plan,"confirmed":true})),
            )
            .await
            .map_err(Into::into)
    }
    .await;
    finish(&job, result, "applied");
}
async fn cleanup(transport: Transport, preview: Option<String>, export: Option<String>) {
    for path in preview
        .map(|id| format!("/api/2026-10-14/admin/imports/{id}"))
        .into_iter()
        .chain(export.map(|id| format!("/api/2026-10-14/admin/exports/{id}/cancel")))
    {
        let _ = transport
            .request_json::<(), Value>(HttpMethod::DELETE, &path, None)
            .await;
    }
}
pub fn cancel(job: Arc<Mutex<Job>>) {
    let owned = job.lock().ok().map(|mut state| {
        state.cancelled.store(true, Ordering::Release);
        if state.view.stage != "applying" {
            state.view.stage = "cancelled";
            state.view.result = None;
        }
        (
            state.connection.transport.clone(),
            state.preview.clone(),
            state.export.clone(),
        )
    });
    if let Some((transport, preview, export)) = owned {
        tauri::async_runtime::spawn(cleanup(transport, preview, export));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_links_and_large_archives_without_mutating_source() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.zip");
        fs::write(&path, b"zip").unwrap();
        assert!(selected_zip(&path).is_ok());
        let link = dir.path().join("link.zip");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert!(selected_zip(&link).is_err());
        let huge = dir.path().join("huge.zip");
        File::create(&huge)
            .unwrap()
            .set_len(MAX_ARCHIVE + 1)
            .unwrap();
        assert!(selected_zip(&huge).is_err());
        assert_eq!(fs::read(path).unwrap(), b"zip");
    }
    #[test]
    fn publication_never_overwrites_racing_destination_and_cleans_temporary() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("saved.zip");
        let (output, mut file) = Output::create(&target).unwrap();
        file.write_all(b"ours").unwrap();
        drop(file);
        fs::write(&target, b"user").unwrap();
        assert!(output.publish().is_err());
        assert_eq!(fs::read(&target).unwrap(), b"user");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
        let out = dir.path().join("ok.zip");
        let (output, file) = Output::create(&out).unwrap();
        drop(file);
        output.publish().unwrap();
        assert_eq!(fs::metadata(out).unwrap().mode() & 0o777, 0o600);
    }
    #[test]
    fn commands_reject_paths_urls_and_unknown_fields() {
        assert!(
            serde_json::from_value::<AdminCommand>(json!({"action":"import","path":"/tmp/file"}))
                .is_err()
        );
        assert!(
            serde_json::from_value::<AdminCommand>(json!({"action":"export","url":"https://bad"}))
                .is_err()
        );
        assert!(
            serde_json::from_value::<AdminCommand>(
                json!({"action":"save","id":"x","url":"https://bad"})
            )
            .is_err()
        );
        assert!(matches!(
            serde_json::from_value::<AdminCommand>(json!({"action":"import"})),
            Ok(AdminCommand::Import {})
        ));
        assert!(matches!(
            serde_json::from_value::<AdminCommand>(json!({"action":"export"})),
            Ok(AdminCommand::Export {})
        ));
    }
}
