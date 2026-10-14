//! One disk-backed export worker per server; HTTP cancellation cannot free its slot.
use crate::{
    api::{ApiError, AppState},
    archive_api::ExportReady,
};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use serde::Serialize;
use std::{
    collections::HashMap,
    path::{Path as FsPath, PathBuf},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, SystemTime},
};

const TTL: Duration = Duration::from_secs(15 * 60);
const MAX_DB: u64 = 256 * 1024 * 1024;
const MAX_DISK: u64 = 2 * 1024 * 1024 * 1024;
static JOBS: OnceLock<Mutex<HashMap<PathBuf, HashMap<String, Job>>>> = OnceLock::new();
static SLOTS: OnceLock<Mutex<HashMap<PathBuf, Arc<tokio::sync::Semaphore>>>> = OnceLock::new();
#[derive(Clone, Serialize)]
struct Status {
    export_id: String,
    state: &'static str,
    bytes: u64,
    entries: u64,
    code: Option<&'static str>,
}
struct Job {
    status: Status,
    cancelled: Arc<AtomicBool>,
    created: SystemTime,
}
struct SnapshotGuard(PathBuf);
impl Drop for SnapshotGuard {
    fn drop(&mut self) {
        for path in [
            self.0.clone(),
            PathBuf::from(format!("{}-wal", self.0.display())),
            PathBuf::from(format!("{}-shm", self.0.display())),
        ] {
            let _ = std::fs::remove_file(path);
        }
    }
}
fn jobs() -> &'static Mutex<HashMap<PathBuf, HashMap<String, Job>>> {
    JOBS.get_or_init(Default::default)
}
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/admin/exports/{id}/status", get(status))
        .route(
            "/api/v1/admin/exports/{id}/cancel",
            axum::routing::delete(cancel),
        )
}
fn valid(id: &str) -> Result<(), ApiError> {
    if uuid::Uuid::parse_str(id).is_ok() {
        Ok(())
    } else {
        Err(ApiError(StatusCode::NOT_FOUND, "EXPORT_NOT_FOUND"))
    }
}
fn cleanup(dir: &FsPath, entries: &mut HashMap<String, Job>) {
    let now = SystemTime::now();
    let expired: Vec<_> = entries
        .iter()
        .filter(|(_, job)| {
            job.status.state != "building"
                && now.duration_since(job.created).is_ok_and(|age| age >= TTL)
        })
        .map(|(id, _)| id.clone())
        .collect();
    for id in expired {
        entries.remove(&id);
        let _ = std::fs::remove_file(dir.join(format!("{id}.zip")));
    }
    if let Ok(files) = std::fs::read_dir(dir) {
        for entry in files.take(4096).flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            let id = [
                ".zip",
                ".partial",
                ".data.partial",
                ".snapshot.sqlite",
                ".snapshot.sqlite-wal",
                ".snapshot.sqlite-shm",
            ]
            .iter()
            .find_map(|ext| {
                name.strip_suffix(ext)
                    .filter(|id| uuid::Uuid::parse_str(id).is_ok())
            });
            let Some(id) = id else { continue };
            if entries.contains_key(id) {
                continue;
            }
            if std::fs::symlink_metadata(entry.path()).is_ok_and(|m| {
                m.is_file()
                    && !m.file_type().is_symlink()
                    && m.modified()
                        .ok()
                        .and_then(|t| now.duration_since(t).ok())
                        .is_some_and(|age| age >= TTL)
            }) {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
}
pub fn initialize(state: &AppState) -> Result<(), ApiError> {
    let dir = crate::archive_api::export_dir(state)?;
    let mut records = jobs()
        .lock()
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_BUSY"))?;
    cleanup(&dir, records.entry(dir.clone()).or_default());
    Ok(())
}
pub(crate) async fn start(state: AppState) -> Result<(StatusCode, Json<ExportReady>), ApiError> {
    let dir = crate::archive_api::export_dir(&state)?;
    let id = uuid::Uuid::new_v4().to_string();
    let permit = SLOTS
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_BUSY"))?
        .entry(dir.clone())
        .or_insert_with(|| Arc::new(tokio::sync::Semaphore::new(1)))
        .clone()
        .try_acquire_owned()
        .map_err(|_| ApiError(StatusCode::TOO_MANY_REQUESTS, "EXPORT_BUSY"))?;
    let flag = Arc::new(AtomicBool::new(false));
    {
        let mut all = jobs()
            .lock()
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_BUSY"))?;
        let entries = all.entry(dir.clone()).or_default();
        cleanup(&dir, entries);
        if entries.len() >= 6 {
            return Err(ApiError(
                StatusCode::TOO_MANY_REQUESTS,
                "EXPORT_STAGING_FULL",
            ));
        }
        // Fail closed; unknown files count but are never deleted.
        let mut usage = 0u64;
        for (index, entry) in std::fs::read_dir(&dir)
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?
            .enumerate()
        {
            if index >= 4096 {
                return Err(ApiError(
                    StatusCode::INSUFFICIENT_STORAGE,
                    "EXPORT_STAGING_FULL",
                ));
            }
            let m = std::fs::symlink_metadata(
                entry
                    .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?
                    .path(),
            )
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
            if !m.is_file() || m.file_type().is_symlink() {
                return Err(ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"));
            }
            usage = usage.saturating_add(m.len());
        }
        if usage.saturating_add(MAX_DB + 64 * 1024 * 1024 + 1024 * 1024 * 1024) > MAX_DISK {
            return Err(ApiError(
                StatusCode::INSUFFICIENT_STORAGE,
                "EXPORT_STAGING_FULL",
            ));
        }
        entries.insert(
            id.clone(),
            Job {
                status: Status {
                    export_id: id.clone(),
                    state: "building",
                    bytes: 0,
                    entries: 0,
                    code: None,
                },
                cancelled: flag.clone(),
                created: SystemTime::now(),
            },
        );
    }
    let worker_id = id.clone();
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let snapshot = dir.join(format!("{worker_id}.snapshot.sqlite"));
        let _guard = SnapshotGuard(snapshot.clone());
        let result = chill_domain::snapshot::coherent_snapshot(
            &state.config.data_dir.join("catalog.sqlite"),
            &snapshot,
            &flag,
            MAX_DB,
        )
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_SNAPSHOT"))
        .and_then(|_| {
            crate::archive_api::build_export(&state, worker_id.clone(), &snapshot, flag.clone())
        });
        if let Ok(mut all) = jobs().lock() {
            if let Some(job) = all.get_mut(&dir).and_then(|j| j.get_mut(&worker_id)) {
                match result {
                    Ok(ready) if !flag.load(Ordering::Acquire) => {
                        job.status = Status {
                            export_id: ready.export_id,
                            state: "ready",
                            bytes: ready.bytes,
                            entries: ready.entries,
                            code: None,
                        }
                    }
                    Ok(_) => {
                        let _ = std::fs::remove_file(dir.join(format!("{worker_id}.zip")));
                        job.status.state = "cancelled";
                    }
                    Err(error) => {
                        job.status.state = if flag.load(Ordering::Acquire) {
                            "cancelled"
                        } else {
                            "failed"
                        };
                        job.status.code = Some(error.1);
                    }
                }
            }
        }
    });
    Ok((
        StatusCode::ACCEPTED,
        Json(ExportReady {
            export_id: id,
            state: "building",
            bytes: 0,
            entries: 0,
        }),
    ))
}
async fn status(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Status>, ApiError> {
    valid(&id)?;
    let dir = crate::archive_api::export_dir(&state)?;
    let mut all = jobs()
        .lock()
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_BUSY"))?;
    let entries = all.entry(dir.clone()).or_default();
    cleanup(&dir, entries);
    let job = entries
        .get(&id)
        .ok_or(ApiError(StatusCode::NOT_FOUND, "EXPORT_NOT_FOUND"))?;
    Ok(Json(job.status.clone()))
}
async fn cancel(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    valid(&id)?;
    let dir = crate::archive_api::export_dir(&state)?;
    let mut all = jobs()
        .lock()
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_BUSY"))?;
    let job = all
        .get_mut(&dir)
        .and_then(|j| j.get_mut(&id))
        .ok_or(ApiError(StatusCode::NOT_FOUND, "EXPORT_NOT_FOUND"))?;
    job.cancelled.store(true, Ordering::Release);
    if job.status.state != "building" {
        let _ = std::fs::remove_file(dir.join(format!("{id}.zip")));
        job.status.state = "cancelled";
    }
    Ok(StatusCode::NO_CONTENT)
}
