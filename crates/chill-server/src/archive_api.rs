//! Staged, immutable administration exports.  Import is deliberately separate.
use crate::api::{ApiError, AppState};
use axum::{
    Json, Router,
    body::Body,
    extract::{DefaultBodyLimit, Path, State},
    http::{StatusCode, header},
    response::Response,
    routing::{delete, get, post},
};
use chill_domain::import::{ArchiveDocument, ImportMapping, ImportResult, PreparedImport};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    io::{Read, Write},
    path::{Path as FsPath, PathBuf},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, SystemTime},
};
use tokio::io::AsyncReadExt;
use tokio_util::io::ReaderStream;
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

const MAX_COMPRESSED: u64 = 1024 * 1024 * 1024;
const MAX_DECOMPRESSED: u64 = 2 * 1024 * 1024 * 1024;
const MAX_ENTRIES: u64 = 20_000;
const MAX_MANIFEST: usize = 64 * 1024 * 1024;
const MAX_DATA: usize = 64 * 1024 * 1024;
const MAX_PREVIEWS: usize = 6;
const PREVIEW_TTL: Duration = Duration::from_secs(15 * 60);
const MAX_STAGING_DISK: u64 = MAX_COMPRESSED * 2;

/// Deliberately process-local: previews are only short lived, read-only staging
/// handles.  No import decision is persisted until the later apply endpoint.
static PREVIEWS: OnceLock<Mutex<HashMap<PathBuf, HashMap<String, Preview>>>> = OnceLock::new();

struct Preview {
    path: PathBuf,
    created: SystemTime,
    busy: Arc<AtomicBool>,
    plan: Option<StoredPlan>,
}
#[derive(Clone)]
struct StoredPlan {
    id: String,
    path: PathBuf,
    root_revision: u64,
}
#[derive(Serialize)]
struct PlanReady {
    plan_id: String,
    epoch: u64,
    root_revision: u64,
    conflicts: Vec<String>,
    conflict_count: usize,
    profiles: BTreeMap<String, String>,
    already_imported: bool,
    apply_supported: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ApplyRequest {
    plan_id: String,
    confirmed: bool,
}
struct PreviewGuard(Arc<AtomicBool>);
impl Drop for PreviewGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

fn reserve_preview(
    dir: &FsPath,
    id: &str,
) -> Result<(PathBuf, Option<StoredPlan>, PreviewGuard), ApiError> {
    if uuid::Uuid::parse_str(id).is_err() {
        return Err(ApiError(StatusCode::NOT_FOUND, "IMPORT_NOT_FOUND"));
    }
    let mut records = previews()
        .lock()
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "BUSY"))?;
    cleanup_previews(dir, &mut records);
    let preview = records
        .get(dir)
        .and_then(|p| p.get(id))
        .ok_or(ApiError(StatusCode::NOT_FOUND, "IMPORT_NOT_FOUND"))?;
    if preview
        .busy
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err(ApiError(StatusCode::CONFLICT, "IMPORT_BUSY"));
    }
    Ok((
        preview.path.clone(),
        preview.plan.clone(),
        PreviewGuard(preview.busy.clone()),
    ))
}

struct PartialFile(Option<PathBuf>);
impl PartialFile {
    fn keep(mut self) {
        self.0.take();
    }
}
impl Drop for PartialFile {
    fn drop(&mut self) {
        if let Some(path) = &self.0 {
            let _ = fs::remove_file(path);
        }
    }
}
// Reservations outlive cancelled async waits because ownership follows the
// actual blocking write/inspection, not the HTTP task.
static RESERVED: OnceLock<Mutex<HashMap<PathBuf, u64>>> = OnceLock::new();
pub(crate) struct DiskReservation {
    dir: PathBuf,
    bytes: u64,
}
impl Drop for DiskReservation {
    fn drop(&mut self) {
        if let Ok(mut ledger) = RESERVED.get_or_init(Default::default).lock() {
            let bytes = ledger.entry(self.dir.clone()).or_default();
            *bytes = bytes.saturating_sub(self.bytes);
        }
    }
}
pub(crate) fn reserve_disk(dir: &FsPath, bytes: u64) -> Result<DiskReservation, ApiError> {
    let mut ledger = RESERVED
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "IMPORT_STORAGE"))?;
    let shared = dir
        .file_name()
        .is_some_and(|n| n == "imports" || n == "exports");
    let key = if shared {
        dir.parent()
            .ok_or(ApiError(StatusCode::SERVICE_UNAVAILABLE, "IMPORT_STORAGE"))?
    } else {
        dir
    };
    let pending = ledger.entry(key.to_path_buf()).or_default();
    let usage = if shared {
        let mut total = 0u64;
        for name in ["imports", "exports"] {
            let path = key.join(name);
            match fs::symlink_metadata(&path) {
                Ok(m) if m.is_dir() && !m.file_type().is_symlink() => {
                    total = total.saturating_add(staging_usage(&path)?)
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                _ => return Err(ApiError(StatusCode::SERVICE_UNAVAILABLE, "IMPORT_STORAGE")),
            }
        }
        total
    } else {
        staging_usage(dir)?
    };
    if usage.saturating_add(*pending).saturating_add(bytes) > MAX_STAGING_DISK {
        return Err(ApiError(
            StatusCode::INSUFFICIENT_STORAGE,
            "IMPORT_STAGING_FULL",
        ));
    }
    *pending += bytes;
    Ok(DiskReservation {
        dir: key.to_path_buf(),
        bytes,
    })
}
struct StagedUpload {
    dir: PathBuf,
    id: String,
    kept: bool,
    cancelled: Arc<AtomicBool>,
}
impl Drop for StagedUpload {
    fn drop(&mut self) {
        if !self.kept {
            self.cancelled.store(true, Ordering::Release);
            if let Ok(mut records) = previews().lock()
                && let Some(items) = records.get_mut(&self.dir)
            {
                items.remove(&self.id);
            }
            for ext in ["partial", "zip"] {
                let _ = fs::remove_file(self.dir.join(format!("{}.{}", self.id, ext)));
            }
        }
    }
}
static INSPECTION_SLOTS: OnceLock<Mutex<HashMap<PathBuf, Arc<tokio::sync::Semaphore>>>> =
    OnceLock::new();
pub(crate) fn inspection_slot(
    dir: &FsPath,
) -> Result<Arc<tokio::sync::OwnedSemaphorePermit>, ApiError> {
    let mut slots = INSPECTION_SLOTS
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "IMPORT_STORAGE"))?;
    slots
        .entry(dir.to_path_buf())
        .or_insert_with(|| Arc::new(tokio::sync::Semaphore::new(1)))
        .clone()
        .try_acquire_owned()
        .map(Arc::new)
        .map_err(|_| ApiError(StatusCode::TOO_MANY_REQUESTS, "IMPORT_INSPECTION_BUSY"))
}

#[derive(Clone, Serialize)]
pub(crate) struct ExportReady {
    pub(crate) export_id: String,
    pub(crate) bytes: u64,
    pub(crate) entries: u64,
    pub(crate) state: &'static str,
}

#[derive(Serialize)]
struct ImportPreview {
    preview_id: String,
    export_id: String,
    counts: PreviewCounts,
    profiles: Vec<PreviewProfile>,
    roots: [&'static str; 1],
    apply_supported: bool,
}

#[derive(Serialize)]
struct PreviewCounts {
    media: usize,
    sagas: usize,
    personal: usize,
    posters: usize,
}

#[derive(Serialize)]
struct PreviewProfile {
    id: String,
    name: String,
}

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/admin/exports", post(create))
        .route("/admin/exports/{id}", get(download))
        // This endpoint writes the body directly to service-owned staging; it
        // intentionally replaces the router's normal 1 MiB JSON body limit.
        .route(
            "/admin/imports",
            post(stage_and_preview).layer(DefaultBodyLimit::max(MAX_COMPRESSED as usize)),
        )
        .route("/admin/imports/{id}/plan", post(prepare_plan))
        .route("/admin/imports/{id}/apply", post(apply_plan))
        .route("/admin/imports/{id}", delete(cancel_preview))
}

async fn prepare_plan(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<ImportMapping>,
) -> Result<Json<PlanReady>, ApiError> {
    let dir = import_dir(&state)?;
    let work = inspection_slot(&dir)?;
    let (path, previous, _guard) = reserve_preview(&dir, &id)?;
    let block_work = work.clone();
    let document = tokio::task::spawn_blocking(move || {
        let _work = block_work;
        read_archive_document(&path)
    })
    .await
    .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "IMPORT_INVALID"))??;
    let library = state.library.clone();
    let actor_work = work.clone();
    let (plan, root_revision) = state
        .database
        .call(move |store| {
            let _work = actor_work;
            let library = library.lock().map_err(|_| "REQUEST_TIMEOUT")?;
            // Existing primary-only archives remain usable with extra roots.
            // An archive containing a virtual extra-root id needs an explicit
            // root mapping UI; never reinterpret it under the primary mount.
            if document
                .media
                .iter()
                .any(|media| media.relative_path.starts_with(".chillcenter-locations/"))
            {
                return Err("IMPORT_MULTIPLE_LIBRARY_LOCATIONS_UNSUPPORTED".into());
            }
            let root = library.video_root.as_ref().ok_or("IMPORT_ROOT_REQUIRED")?;
            Ok((
                store.prepare_portable_import(document, &request, root)?,
                library.revision,
            ))
        })
        .await?;
    let plan_id = uuid::Uuid::new_v4().to_string();
    let plan_path = dir.join(format!("{plan_id}.plan"));
    let bytes = serde_json::to_vec(&plan)
        .map_err(|_| ApiError(StatusCode::UNPROCESSABLE_ENTITY, "IMPORT_INVALID"))?;
    if bytes.len() > MAX_DATA {
        return Err(ApiError(StatusCode::PAYLOAD_TOO_LARGE, "IMPORT_TOO_LARGE"));
    }
    let plan_guard = PartialFile(Some(plan_path.clone()));
    let _reservation = reserve_disk(&dir, bytes.len() as u64)?;
    crate::config::atomic_private_write(&plan_path, &bytes)
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "IMPORT_STORAGE"))?;
    {
        let mut records = previews()
            .lock()
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "BUSY"))?;
        records
            .get_mut(&dir)
            .and_then(|p| p.get_mut(&id))
            .ok_or(ApiError(StatusCode::NOT_FOUND, "IMPORT_NOT_FOUND"))?
            .plan = Some(StoredPlan {
            id: plan_id.clone(),
            path: plan_path,
            root_revision,
        });
    }
    if let Some(previous) = previous {
        let _ = fs::remove_file(previous.path);
    }
    // Registry now owns this exact plan. Forget only the guard, not a leaked
    // upload path; cleanup_previews reclaims it on expiration/cancellation.
    plan_guard.keep();
    Ok(Json(PlanReady {
        plan_id,
        epoch: plan.epoch,
        root_revision,
        conflicts: plan.conflicts.iter().take(100).cloned().collect(),
        conflict_count: plan.conflicts.len(),
        profiles: plan
            .profiles
            .iter()
            .map(|p| (p.source.clone(), p.id.clone()))
            .collect(),
        already_imported: plan.already_imported,
        apply_supported: true,
    }))
}
fn read_archive_document(path: &FsPath) -> Result<ArchiveDocument, ApiError> {
    let file =
        fs::File::open(path).map_err(|_| ApiError(StatusCode::NOT_FOUND, "IMPORT_NOT_FOUND"))?;
    let mut zip = zip::ZipArchive::new(file)
        .map_err(|_| ApiError(StatusCode::UNPROCESSABLE_ENTITY, "IMPORT_INVALID"))?;
    let bytes = read_zip_limited(&mut zip, "data.json", MAX_DATA)?;
    let document: ArchiveDocument = serde_json::from_slice(&bytes)
        .map_err(|_| ApiError(StatusCode::UNPROCESSABLE_ENTITY, "IMPORT_INVALID"))?;
    document
        .validate()
        .map_err(|_| ApiError(StatusCode::UNPROCESSABLE_ENTITY, "IMPORT_INVALID"))?;
    Ok(document)
}

struct OwnedPosters {
    paths: BTreeMap<String, String>,
    committed: bool,
}
impl OwnedPosters {
    fn keep_after_commit(&mut self) {
        self.committed = true;
    }
}
impl Drop for OwnedPosters {
    fn drop(&mut self) {
        if !self.committed {
            for path in self.paths.values() {
                let _ = fs::remove_file(path);
            }
        }
    }
}
async fn apply_plan(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<ApplyRequest>,
) -> Result<Json<ImportResult>, ApiError> {
    if !request.confirmed {
        return Err(ApiError(
            StatusCode::UNPROCESSABLE_ENTITY,
            "IMPORT_CONFIRMATION_REQUIRED",
        ));
    }
    let dir = import_dir(&state)?;
    let work = inspection_slot(&dir)?;
    let (archive, stored, _guard) = reserve_preview(&dir, &id)?;
    let stored = stored
        .filter(|p| p.id == request.plan_id)
        .ok_or(ApiError(StatusCode::CONFLICT, "IMPORT_PLAN_STALE"))?;
    let cache = state.config.data_dir.join("posters");
    let plan_path = stored.path;
    let block_work = work.clone();
    let (plan, mut posters) = tokio::task::spawn_blocking(move || {
        let _work = block_work;
        let input = fs::File::open(&plan_path)
            .map_err(|_| ApiError(StatusCode::NOT_FOUND, "IMPORT_NOT_FOUND"))?;
        let mut bytes = Vec::new();
        input
            .take(MAX_DATA as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "IMPORT_STORAGE"))?;
        if bytes.len() > MAX_DATA {
            return Err(ApiError(StatusCode::PAYLOAD_TOO_LARGE, "IMPORT_TOO_LARGE"));
        }
        let plan: PreparedImport = serde_json::from_slice(&bytes)
            .map_err(|_| ApiError(StatusCode::UNPROCESSABLE_ENTITY, "IMPORT_INVALID"))?;
        let _ = inspect_archive(&archive, &id)?;
        let mut zip = zip::ZipArchive::new(
            fs::File::open(&archive)
                .map_err(|_| ApiError(StatusCode::NOT_FOUND, "IMPORT_NOT_FOUND"))?,
        )
        .map_err(|_| ApiError(StatusCode::UNPROCESSABLE_ENTITY, "IMPORT_INVALID"))?;
        let mut posters = OwnedPosters {
            paths: BTreeMap::new(),
            committed: false,
        };
        let refs = plan
            .media
            .iter()
            .filter(|p| {
                !p.existing
                    || plan.policy == chill_domain::import::ConflictPolicy::ReplaceConflicting
            })
            .filter_map(|p| p.poster_ref.as_ref())
            .chain(
                plan.sagas
                    .iter()
                    .filter(|p| {
                        !p.existing
                            || plan.policy
                                == chill_domain::import::ConflictPolicy::ReplaceConflicting
                    })
                    .filter_map(|p| p.poster_ref.as_ref()),
            )
            .chain(
                plan.personal
                    .iter()
                    .flat_map(|p| p.playlists.iter())
                    .filter_map(|p| p.poster_ref.as_ref()),
            )
            .collect::<HashSet<_>>();
        for reference in refs {
            let bytes = read_zip_limited(&mut zip, reference, 8 * 1024 * 1024)?;
            let path = chill_catalog::poster::store(&bytes, &cache)
                .map_err(|_| ApiError(StatusCode::UNPROCESSABLE_ENTITY, "IMPORT_POSTER_INVALID"))?;
            posters.paths.insert(reference.clone(), path);
        }
        Ok::<_, ApiError>((plan, posters))
    })
    .await
    .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "IMPORT_INVALID"))??;
    let library = state.library.clone();
    let actor_work = work.clone();
    // Ownership follows the transaction, not the HTTP response: a cancelled
    // client must never delete artwork referenced by an already committed DB.
    let result = state
        .database
        .call(move |store| {
            let _work = actor_work;
            let library = library.lock().map_err(|_| "REQUEST_TIMEOUT")?;
            if library.revision != stored.root_revision {
                return Err("IMPORT_PLAN_STALE".into());
            }
            let result = store.apply_portable_import(&plan, &posters.paths)?;
            if !result.already_imported {
                posters.keep_after_commit();
            }
            Ok(result)
        })
        .await?;
    Ok(Json(result))
}

fn import_dir(state: &AppState) -> Result<PathBuf, ApiError> {
    let path = state.config.data_dir.join("imports");
    fs::create_dir_all(&path)
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "IMPORT_STORAGE"))?;
    let metadata = fs::symlink_metadata(&path)
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "IMPORT_STORAGE"))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(ApiError(StatusCode::SERVICE_UNAVAILABLE, "IMPORT_STORAGE"));
    }
    Ok(path)
}
/// Recover expired service staging even before the first administration visit.
/// The source ZIP selected by a user is never part of this directory.
pub fn initialize_staging(state: &AppState) -> Result<(), ApiError> {
    let dir = import_dir(state)?;
    let mut records = previews()
        .lock()
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "IMPORT_STORAGE"))?;
    cleanup_previews(&dir, &mut records);
    Ok(())
}

fn previews() -> &'static Mutex<HashMap<PathBuf, HashMap<String, Preview>>> {
    PREVIEWS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cleanup_previews(dir: &FsPath, records: &mut HashMap<PathBuf, HashMap<String, Preview>>) {
    let previews = records.entry(dir.to_path_buf()).or_default();
    let now = SystemTime::now();
    let expired: Vec<_> = previews
        .iter()
        .filter_map(|(id, preview)| {
            if preview.busy.load(Ordering::Acquire) {
                return None;
            }
            now.duration_since(preview.created)
                .ok()
                .filter(|age| *age >= PREVIEW_TTL)
                .map(|_| id.clone())
        })
        .collect();
    for id in expired {
        if let Some(preview) = previews.remove(&id) {
            let _ = fs::remove_file(preview.path);
            if let Some(plan) = preview.plan {
                let _ = fs::remove_file(plan.path);
            }
        }
    }
    // After restart there is no registry. Recover only recognizable,
    // expired service-owned staging; unrelated files are never deleted.
    let active: HashSet<_> = previews
        .iter()
        .flat_map(|(id, p)| {
            [
                Some(format!("{id}.zip")),
                Some(format!("{id}.partial")),
                p.plan.as_ref().map(|p| format!("{}.plan", p.id)),
            ]
        })
        .flatten()
        .collect();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.take(4096).flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if !managed_staging(name) || active.contains(name) {
                continue;
            }
            if let Ok(meta) = fs::symlink_metadata(entry.path())
                && meta.is_file()
                && !meta.file_type().is_symlink()
                && meta
                    .modified()
                    .ok()
                    .and_then(|time| now.duration_since(time).ok())
                    .is_some_and(|age| age >= PREVIEW_TTL)
            {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
}

fn managed_staging(name: &str) -> bool {
    let name = name.strip_prefix(".chillcenter-").unwrap_or(name);
    name.rsplit_once('.').is_some_and(|(id, ext)| {
        uuid::Uuid::parse_str(id).is_ok() && matches!(ext, "partial" | "zip" | "plan" | "tmp")
    })
}
fn staging_usage(dir: &FsPath) -> Result<u64, ApiError> {
    let entries = fs::read_dir(dir)
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "IMPORT_STORAGE"))?;
    let mut bytes = 0u64;
    for (index, entry) in entries.enumerate() {
        if index >= 4096 {
            return Err(ApiError(
                StatusCode::INSUFFICIENT_STORAGE,
                "IMPORT_STAGING_FULL",
            ));
        }
        let entry =
            entry.map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "IMPORT_STORAGE"))?;
        let meta = fs::symlink_metadata(entry.path())
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "IMPORT_STORAGE"))?;
        if !meta.is_file() || meta.file_type().is_symlink() {
            return Err(ApiError(StatusCode::SERVICE_UNAVAILABLE, "IMPORT_STORAGE"));
        }
        bytes = bytes.saturating_add(meta.len());
    }
    Ok(bytes)
}

async fn stage_and_preview(
    State(state): State<AppState>,
    body: Body,
) -> Result<Json<ImportPreview>, ApiError> {
    let dir = import_dir(&state)?;
    let work = inspection_slot(&dir)?;
    let id = uuid::Uuid::new_v4().to_string();
    let mut stage_guard = StagedUpload {
        dir: dir.clone(),
        id: id.clone(),
        kept: false,
        cancelled: Arc::new(AtomicBool::new(false)),
    };
    {
        let mut records = previews()
            .lock()
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "BUSY"))?;
        cleanup_previews(&dir, &mut records);
        let active = records.get(&dir).map_or(0, HashMap::len);
        if active >= MAX_PREVIEWS {
            return Err(ApiError(
                StatusCode::TOO_MANY_REQUESTS,
                "IMPORT_PREVIEW_BUSY",
            ));
        }
        records.get_mut(&dir).unwrap().insert(
            id.clone(),
            Preview {
                path: dir.join(format!("{id}.zip")),
                created: SystemTime::now(),
                busy: Arc::new(AtomicBool::new(true)),
                plan: None,
            },
        );
    }
    let partial = dir.join(format!("{id}.partial"));
    let staged = dir.join(format!("{id}.zip"));
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    use std::os::unix::fs::OpenOptionsExt;
    #[cfg(unix)]
    options.mode(0o600);
    let mut output = options
        .open(&partial)
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "IMPORT_STORAGE"))?;
    let mut bytes = 0u64;
    let mut stream = body.into_data_stream();
    while let Some(chunk) = stream.next().await {
        let chunk =
            chunk.map_err(|_| ApiError(StatusCode::BAD_REQUEST, "IMPORT_UPLOAD_INVALID"))?;
        bytes = bytes
            .checked_add(chunk.len() as u64)
            .ok_or(ApiError(StatusCode::PAYLOAD_TOO_LARGE, "IMPORT_TOO_LARGE"))?;
        if bytes > MAX_COMPRESSED {
            let _ = tokio::fs::remove_file(&partial).await;
            return Err(ApiError(StatusCode::PAYLOAD_TOO_LARGE, "IMPORT_TOO_LARGE"));
        }
        let reservation = reserve_disk(&dir, chunk.len() as u64)?;
        let block_work = work.clone();
        output = tokio::task::spawn_blocking(move || {
            let _work = block_work;
            let _reservation = reservation;
            output
                .write_all(&chunk)
                .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "IMPORT_STORAGE"))?;
            Ok::<_, ApiError>(output)
        })
        .await
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "IMPORT_STORAGE"))??;
    }
    let block_work = work.clone();
    tokio::task::spawn_blocking(move || {
        let _work = block_work;
        output.flush().and_then(|()| output.sync_all())
    })
    .await
    .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "IMPORT_STORAGE"))?
    .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "IMPORT_STORAGE"))?;
    fs::rename(&partial, &staged)
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "IMPORT_STORAGE"))?;
    let inspect_id = id.clone();
    let inspect_permit = work.clone();
    let cancelled = stage_guard.cancelled.clone();
    let checked = tokio::task::spawn_blocking({
        let path = staged.clone();
        move || {
            let _permit = inspect_permit;
            inspect_archive_cancellable(&path, &inspect_id, &cancelled)
        }
    })
    .await
    .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "IMPORT_INVALID"))?;
    let summary = match checked {
        Ok(summary) => summary,
        Err(error) => {
            let _ = fs::remove_file(&staged);
            return Err(error);
        }
    };
    let mut records = previews()
        .lock()
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "BUSY"))?;
    cleanup_previews(&dir, &mut records);
    records.entry(dir).or_default().insert(
        id.clone(),
        Preview {
            path: staged,
            created: SystemTime::now(),
            busy: Arc::new(AtomicBool::new(false)),
            plan: None,
        },
    );
    stage_guard.kept = true;
    Ok(Json(ImportPreview {
        preview_id: id,
        export_id: summary.export_id,
        counts: summary.counts,
        profiles: summary.profiles,
        roots: ["library"],
        apply_supported: true,
    }))
}

async fn cancel_preview(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    if uuid::Uuid::parse_str(&id).is_err() {
        return Err(ApiError(StatusCode::NOT_FOUND, "IMPORT_NOT_FOUND"));
    }
    let dir = import_dir(&state)?;
    let (_, _, _guard) = reserve_preview(&dir, &id)?;
    let mut records = previews()
        .lock()
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "BUSY"))?;
    cleanup_previews(&dir, &mut records);
    let preview = records
        .get_mut(&dir)
        .and_then(|items| items.remove(&id))
        .ok_or(ApiError(StatusCode::NOT_FOUND, "IMPORT_NOT_FOUND"))?;
    fs::remove_file(preview.path)
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "IMPORT_STORAGE"))?;
    if let Some(plan) = preview.plan {
        let _ = fs::remove_file(plan.path);
    }
    Ok(StatusCode::NO_CONTENT)
}

struct ArchiveSummary {
    export_id: String,
    counts: PreviewCounts,
    profiles: Vec<PreviewProfile>,
}

fn inspect_archive(path: &FsPath, preview_id: &str) -> Result<ArchiveSummary, ApiError> {
    inspect_archive_cancellable(path, preview_id, &AtomicBool::new(false))
}
fn inspect_archive_cancellable(
    path: &FsPath,
    preview_id: &str,
    cancelled: &AtomicBool,
) -> Result<ArchiveSummary, ApiError> {
    let check_cancel = || {
        if cancelled.load(Ordering::Acquire) {
            Err(ApiError(StatusCode::REQUEST_TIMEOUT, "IMPORT_CANCELLED"))
        } else {
            Ok(())
        }
    };
    check_cancel()?;
    #[cfg(test)]
    if let Some(gate) = INSPECTION_GATES
        .get_or_init(Default::default)
        .lock()
        .unwrap()
        .get(path.parent().unwrap())
        .cloned()
    {
        gate.entered.store(true, Ordering::Release);
        let mut released = gate.released.lock().unwrap();
        while !*released {
            released = gate.wake.wait(released).unwrap();
        }
    }
    let file = fs::File::open(path)
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "IMPORT_STORAGE"))?;
    let mut zip = zip::ZipArchive::new(file)
        .map_err(|_| ApiError(StatusCode::UNPROCESSABLE_ENTITY, "IMPORT_INVALID"))?;
    if zip.len() > MAX_ENTRIES as usize {
        return Err(ApiError(StatusCode::PAYLOAD_TOO_LARGE, "IMPORT_TOO_LARGE"));
    }
    let mut names = HashSet::new();
    let mut total = 0u64;
    for index in 0..zip.len() {
        check_cancel()?;
        let entry = zip
            .by_index(index)
            .map_err(|_| ApiError(StatusCode::UNPROCESSABLE_ENTITY, "IMPORT_INVALID"))?;
        let name = entry
            .name()
            .map_err(|_| ApiError(StatusCode::UNPROCESSABLE_ENTITY, "IMPORT_INVALID"))?;
        if !safe_archive_name(&name)
            || !names.insert(name.to_string())
            || entry.is_dir()
            || entry.is_symlink()
            || entry.encrypted()
            || name.ends_with(".zip")
        {
            return Err(ApiError(StatusCode::UNPROCESSABLE_ENTITY, "IMPORT_INVALID"));
        }
        total = total
            .checked_add(entry.size())
            .ok_or(ApiError(StatusCode::PAYLOAD_TOO_LARGE, "IMPORT_TOO_LARGE"))?;
        if total > MAX_DECOMPRESSED {
            return Err(ApiError(StatusCode::PAYLOAD_TOO_LARGE, "IMPORT_TOO_LARGE"));
        }
    }
    let manifest = read_zip_limited(&mut zip, "manifest.json", MAX_MANIFEST)?;
    let data = read_zip_limited(&mut zip, "data.json", MAX_DATA)?;
    let manifest: serde_json::Value = serde_json::from_slice(&manifest)
        .map_err(|_| ApiError(StatusCode::UNPROCESSABLE_ENTITY, "IMPORT_INVALID"))?;
    let document: serde_json::Value = serde_json::from_slice(&data)
        .map_err(|_| ApiError(StatusCode::UNPROCESSABLE_ENTITY, "IMPORT_INVALID"))?;
    let version = manifest.get("format_version").and_then(|v| v.as_u64());
    let export_id = manifest
        .get("export_id")
        .and_then(|v| v.as_str())
        .filter(|id| uuid::Uuid::parse_str(id).is_ok())
        .ok_or(ApiError(StatusCode::UNPROCESSABLE_ENTITY, "IMPORT_INVALID"))?;
    if version != Some(1)
        || !matches!(
            document.get("format_version").and_then(|v| v.as_u64()),
            Some(1 | 2)
        )
        || document.get("export_id").and_then(|v| v.as_str()) != Some(export_id)
        || export_id == preview_id
    {
        return Err(ApiError(StatusCode::UNPROCESSABLE_ENTITY, "IMPORT_INVALID"));
    }
    let entries = manifest
        .get("entries")
        .and_then(|v| v.as_array())
        .ok_or(ApiError(StatusCode::UNPROCESSABLE_ENTITY, "IMPORT_INVALID"))?;
    let mut checksum_names = HashSet::new();
    for item in entries {
        check_cancel()?;
        let name = item
            .get("reference")
            .and_then(|v| v.as_str())
            .filter(|n| safe_archive_name(n))
            .ok_or(ApiError(StatusCode::UNPROCESSABLE_ENTITY, "IMPORT_INVALID"))?;
        let expected = item
            .get("sha256")
            .and_then(|v| v.as_str())
            .filter(|v| v.len() == 64 && v.bytes().all(|b| b.is_ascii_hexdigit()))
            .ok_or(ApiError(StatusCode::UNPROCESSABLE_ENTITY, "IMPORT_INVALID"))?;
        if !checksum_names.insert(name) {
            return Err(ApiError(StatusCode::UNPROCESSABLE_ENTITY, "IMPORT_INVALID"));
        }
        let bytes = read_zip_limited(&mut zip, name, MAX_DATA)?;
        if format!("{:x}", Sha256::digest(&bytes)) != expected.to_ascii_lowercase() {
            return Err(ApiError(
                StatusCode::UNPROCESSABLE_ENTITY,
                "IMPORT_CHECKSUM",
            ));
        }
    }
    if !checksum_names.contains("data.json")
        || names.len() != checksum_names.len() + 1
        || !names
            .iter()
            .all(|name| name == "manifest.json" || checksum_names.contains(name.as_str()))
    {
        return Err(ApiError(StatusCode::UNPROCESSABLE_ENTITY, "IMPORT_INVALID"));
    }
    let profiles = document
        .get("profiles")
        .and_then(|v| v.as_array())
        .ok_or(ApiError(StatusCode::UNPROCESSABLE_ENTITY, "IMPORT_INVALID"))?;
    let media = required_array(&document, "media")?;
    let sagas = required_array(&document, "sagas")?;
    let personal = required_array(&document, "personal")?;
    let _leases = required_array(&document, "leases")?;
    let mut out_profiles = Vec::with_capacity(profiles.len());
    for profile in profiles {
        out_profiles.push(PreviewProfile {
            id: profile
                .get("id")
                .and_then(|v| v.as_str())
                .filter(|v| !v.is_empty())
                .ok_or(ApiError(StatusCode::UNPROCESSABLE_ENTITY, "IMPORT_INVALID"))?
                .to_owned(),
            name: profile
                .get("name")
                .and_then(|v| v.as_str())
                .filter(|v| !v.is_empty())
                .ok_or(ApiError(StatusCode::UNPROCESSABLE_ENTITY, "IMPORT_INVALID"))?
                .to_owned(),
        });
    }
    Ok(ArchiveSummary {
        export_id: export_id.to_owned(),
        counts: PreviewCounts {
            media: media.len(),
            sagas: sagas.len(),
            personal: personal.len(),
            posters: entries.len().saturating_sub(1),
        },
        profiles: out_profiles,
    })
}

fn required_array<'a>(
    document: &'a serde_json::Value,
    name: &str,
) -> Result<&'a Vec<serde_json::Value>, ApiError> {
    document
        .get(name)
        .and_then(|v| v.as_array())
        .ok_or(ApiError(StatusCode::UNPROCESSABLE_ENTITY, "IMPORT_INVALID"))
}
fn safe_archive_name(name: &str) -> bool {
    !name.is_empty()
        && name.is_ascii()
        && !name.contains('\\')
        && name
            .split('/')
            .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
}
fn read_zip_limited(
    zip: &mut zip::ZipArchive<fs::File>,
    name: &str,
    limit: usize,
) -> Result<Vec<u8>, ApiError> {
    let entry = zip
        .by_name(name)
        .map_err(|_| ApiError(StatusCode::UNPROCESSABLE_ENTITY, "IMPORT_INVALID"))?;
    if entry.size() > limit as u64 {
        return Err(ApiError(StatusCode::PAYLOAD_TOO_LARGE, "IMPORT_TOO_LARGE"));
    }
    let mut bytes = Vec::with_capacity(entry.size() as usize);
    entry
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ApiError(StatusCode::UNPROCESSABLE_ENTITY, "IMPORT_INVALID"))?;
    if bytes.len() > limit {
        return Err(ApiError(StatusCode::PAYLOAD_TOO_LARGE, "IMPORT_TOO_LARGE"));
    }
    Ok(bytes)
}

pub(crate) fn export_dir(state: &AppState) -> Result<PathBuf, ApiError> {
    let path = state.config.data_dir.join("exports");
    fs::create_dir_all(&path)
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
    Ok(path)
}

async fn create(
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<ExportReady>), ApiError> {
    crate::export_jobs::start(state).await
}

struct ExportFile {
    file: fs::File,
    dir: PathBuf,
    maximum: u64,
    cancelled: Arc<AtomicBool>,
}
impl Write for ExportFile {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.cancelled.load(Ordering::Acquire) {
            return Err(std::io::Error::from(std::io::ErrorKind::Interrupted));
        }
        use std::io::Seek;
        let position = self.file.stream_position()?;
        if position.saturating_add(bytes.len() as u64) > self.maximum {
            return Err(std::io::Error::from(std::io::ErrorKind::FileTooLarge));
        }
        let _reservation = reserve_disk(&self.dir, bytes.len() as u64)
            .map_err(|_| std::io::Error::from(std::io::ErrorKind::StorageFull))?;
        self.file.write(bytes)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}
impl std::io::Seek for ExportFile {
    fn seek(&mut self, position: std::io::SeekFrom) -> std::io::Result<u64> {
        self.file.seek(position)
    }
}
fn export_file(
    path: &FsPath,
    maximum: u64,
    cancelled: &Arc<AtomicBool>,
) -> Result<ExportFile, ApiError> {
    let mut options = fs::OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    Ok(ExportFile {
        file: options
            .open(path)
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?,
        dir: path.parent().unwrap().to_path_buf(),
        maximum,
        cancelled: cancelled.clone(),
    })
}
pub(crate) fn build_export(
    state: &AppState,
    id: String,
    snapshot: &FsPath,
    cancelled: Arc<AtomicBool>,
) -> Result<ExportReady, ApiError> {
    build_export_files(&state.config.data_dir, id, snapshot, cancelled)
}
/// Single portable archive writer for the service and the read-only legacy CLI.
pub(crate) fn build_export_files(
    data_dir: &FsPath,
    id: String,
    snapshot: &FsPath,
    cancelled: Arc<AtomicBool>,
) -> Result<ExportReady, ApiError> {
    let dir = data_dir.join("exports");
    fs::create_dir_all(&dir)
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
    let partial = dir.join(format!("{id}.partial"));
    let ready = dir.join(format!("{id}.zip"));
    let data_stage = dir.join(format!("{id}.data.partial"));
    let poster_root = data_dir.join("posters");
    let _partial_guard = PartialFile(Some(partial.clone()));
    let _data_guard = PartialFile(Some(data_stage.clone()));
    let ready_guard = PartialFile(Some(ready.clone()));
    let mut store = chill_domain::Store::open_export_snapshot(snapshot)
        .map_err(|_| ApiError(StatusCode::UNPROCESSABLE_ENTITY, "EXPORT_SNAPSHOT"))?;
    let mut file = export_file(&data_stage, MAX_DATA as u64, &cancelled)?;
    let posters = store
        .write_archive_json(&mut file, &id, &poster_root)
        .map_err(|_| {
            ApiError(
                StatusCode::INSUFFICIENT_STORAGE,
                "EXPORT_TOO_LARGE_OR_STORAGE",
            )
        })?;
    file.flush()
        .and_then(|_| file.file.sync_all())
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
    drop(file);
    drop(store);
    let data_size = fs::metadata(&data_stage)
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?
        .len();
    if data_size > MAX_DECOMPRESSED {
        let _ = fs::remove_file(&data_stage);
        return Err(ApiError(StatusCode::PAYLOAD_TOO_LARGE, "EXPORT_TOO_LARGE"));
    }
    let result = (|| -> Result<(u64, u64), ApiError> {
        let file = export_file(&partial, MAX_COMPRESSED, &cancelled)?;
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        let mut checksums =
            vec![serde_json::json!({"reference":"data.json","sha256":hash_file(&data_stage)?})];
        let mut poster_hashes = BTreeMap::new();
        for poster in &posters {
            if cancelled.load(Ordering::Acquire) {
                return Err(ApiError(StatusCode::CONFLICT, "EXPORT_CANCELLED"));
            }
            let source = poster_root.join(&poster.relative);
            let (mut source_file, expected) =
                crate::media_stream::open_indexed(&poster_root, &source)
                    .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_POSTER"))?;
            if expected.size > chill_domain::store::MAX_EXPORT_POSTER_BYTES {
                return Err(ApiError(StatusCode::PAYLOAD_TOO_LARGE, "EXPORT_TOO_LARGE"));
            }
            let mut bytes = Vec::with_capacity(expected.size as usize);
            Read::by_ref(&mut source_file)
                .take(chill_domain::store::MAX_EXPORT_POSTER_BYTES + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_POSTER"))?;
            if bytes.len() as u64 != expected.size
                || crate::media_stream::FileVersion::from_metadata(
                    &source_file
                        .metadata()
                        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_POSTER"))?,
                )
                .ok()
                .as_ref()
                    != Some(&expected)
            {
                return Err(ApiError(
                    StatusCode::PRECONDITION_FAILED,
                    "EXPORT_POSTER_CHANGED",
                ));
            }
            let hash = format!("{:x}", Sha256::digest(&bytes));
            checksums.push(serde_json::json!({"reference":poster.reference,"sha256":hash}));
            poster_hashes.insert(poster.reference.clone(), hash);
        }
        let manifest = serde_json::to_vec(&serde_json::json!({"format_version":1,"export_id":id,"entries":checksums,"videos":false,"leases":false,"secrets":false})).map_err(|_| ApiError(StatusCode::UNPROCESSABLE_ENTITY, "EXPORT_INVALID"))?;
        if manifest.len() > MAX_MANIFEST {
            return Err(ApiError(StatusCode::PAYLOAD_TOO_LARGE, "EXPORT_TOO_LARGE"));
        }
        zip.start_file("manifest.json", options)
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
        zip.write_all(&manifest)
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
        zip.start_file("data.json", options)
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
        let mut data_file = fs::File::open(&data_stage)
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
        std::io::copy(&mut data_file, &mut zip)
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
        // Two passes keep at most one image in memory, regardless of library
        // size. The second checksum refuses an image replaced in between.
        for poster in &posters {
            let source = poster_root.join(&poster.relative);
            let (mut file, version) = crate::media_stream::open_indexed(&poster_root, &source)
                .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_POSTER"))?;
            if version.size > chill_domain::store::MAX_EXPORT_POSTER_BYTES {
                return Err(ApiError(StatusCode::PAYLOAD_TOO_LARGE, "EXPORT_TOO_LARGE"));
            }
            let mut bytes = Vec::with_capacity(version.size as usize);
            Read::by_ref(&mut file)
                .take(chill_domain::store::MAX_EXPORT_POSTER_BYTES + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_POSTER"))?;
            if bytes.len() as u64 != version.size
                || poster_hashes.get(&poster.reference)
                    != Some(&format!("{:x}", Sha256::digest(&bytes)))
            {
                return Err(ApiError(
                    StatusCode::PRECONDITION_FAILED,
                    "EXPORT_POSTER_CHANGED",
                ));
            }
            zip.start_file(&poster.reference, options)
                .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
            zip.write_all(&bytes)
                .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
        }
        let mut file = zip
            .finish()
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
        file.flush()
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
        file.file
            .sync_all()
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
        let size = file
            .file
            .metadata()
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?
            .len();
        if size > MAX_COMPRESSED || (2 + posters.len() as u64) > MAX_ENTRIES {
            return Err(ApiError(StatusCode::PAYLOAD_TOO_LARGE, "EXPORT_TOO_LARGE"));
        }
        Ok((size, 2 + posters.len() as u64))
    })();
    match result {
        Ok((bytes, entries)) => {
            if cancelled.load(Ordering::Acquire) {
                return Err(ApiError(StatusCode::CONFLICT, "EXPORT_CANCELLED"));
            }
            // Never publish a coherent ZIP that the typed importer would reject.
            read_archive_document(&partial)?;
            fs::rename(&partial, &ready)
                .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
            fs::File::open(&dir)
                .and_then(|directory| directory.sync_all())
                .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
            let _ = fs::remove_file(&data_stage);
            ready_guard.keep();
            Ok(ExportReady {
                export_id: id,
                bytes,
                entries,
                state: "ready",
            })
        }
        Err(error) => {
            let _ = fs::remove_file(&partial);
            let _ = fs::remove_file(&data_stage);
            Err(error)
        }
    }
}

fn hash_file(path: &std::path::Path) -> Result<String, ApiError> {
    let mut file = fs::File::open(path)
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let n = std::io::Read::read(&mut file, &mut buffer)
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
        if n == 0 {
            break;
        };
        hash.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

#[cfg(test)]
struct InspectionGate {
    entered: AtomicBool,
    released: Mutex<bool>,
    wake: std::sync::Condvar,
}
#[cfg(test)]
static INSPECTION_GATES: OnceLock<Mutex<HashMap<PathBuf, Arc<InspectionGate>>>> = OnceLock::new();

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;
    use crate::{
        api::{DatabaseActor, router},
        config::{LibrarySettings, ServerConfig},
        playback::PlaybackService,
        tls,
    };
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use std::{
        io::Cursor,
        sync::{Arc, Mutex},
    };
    use tower::ServiceExt;
    #[test]
    fn export_file_bounds_live_disk_and_cancellation_before_writes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("partial");
        let cancelled = Arc::new(AtomicBool::new(false));
        let mut file = export_file(&path, 4, &cancelled).unwrap();
        file.write_all(b"1234").unwrap();
        assert!(file.write_all(b"5").is_err());
        assert_eq!(fs::read(&path).unwrap(), b"1234");
        cancelled.store(true, Ordering::Release);
        assert!(file.write(b"x").is_err());
        let guard = PartialFile(Some(path.clone()));
        drop(file);
        drop(guard);
        assert!(!path.exists());
    }
    #[test]
    fn staging_disk_accounts_orphans_plans_and_inflight_reservations_fail_closed() {
        let dir = tempfile::tempdir().unwrap();
        let orphan = dir.path().join(format!("{}.zip", uuid::Uuid::new_v4()));
        let file = fs::File::create(&orphan).unwrap();
        file.set_len(MAX_STAGING_DISK - 16).unwrap();
        assert_eq!(staging_usage(dir.path()).unwrap(), MAX_STAGING_DISK - 16);
        let held = reserve_disk(dir.path(), 8).unwrap();
        assert!(reserve_disk(dir.path(), 9).is_err());
        drop(held);
        let plan = dir.path().join(format!("{}.plan", uuid::Uuid::new_v4()));
        fs::write(&plan, [0u8; 16]).unwrap();
        assert!(reserve_disk(dir.path(), 1).is_err());
        assert!(staging_usage(&dir.path().join("absent")).is_err());
        fs::remove_file(&plan).unwrap();
        std::os::unix::fs::symlink("/does/not/exist", &plan).unwrap();
        assert!(staging_usage(dir.path()).is_err());
    }
    #[test]
    fn imports_and_exports_share_one_disk_budget_and_reservation_ledger() {
        let root = tempfile::tempdir().unwrap();
        let imports = root.path().join("imports");
        let exports = root.path().join("exports");
        fs::create_dir(&imports).unwrap();
        fs::create_dir(&exports).unwrap();
        fs::File::create(exports.join("unknown-preserved.bin"))
            .unwrap()
            .set_len(MAX_STAGING_DISK - 16)
            .unwrap();
        let held = reserve_disk(&imports, 8).unwrap();
        assert!(reserve_disk(&exports, 9).is_err());
        drop(held);
        assert!(reserve_disk(&exports, 16).is_ok());
        assert!(exports.join("unknown-preserved.bin").exists());
    }
    #[test]
    fn restart_sweep_removes_only_expired_managed_staging_and_preserves_busy_files() {
        let dir = tempfile::tempdir().unwrap();
        let old = SystemTime::now() - PREVIEW_TTL - Duration::from_secs(1);
        let orphan = dir.path().join(format!("{}.zip", uuid::Uuid::new_v4()));
        let plan = dir.path().join(format!("{}.plan", uuid::Uuid::new_v4()));
        let note = dir.path().join("user-note.txt");
        let active_id = uuid::Uuid::new_v4().to_string();
        let active = dir.path().join(format!("{active_id}.partial"));
        for path in [&orphan, &plan, &note, &active] {
            let mut file = fs::File::create(path).unwrap();
            file.write_all(b"fixture").unwrap();
            file.set_times(fs::FileTimes::new().set_modified(old))
                .unwrap();
        }
        let mut records = HashMap::new();
        records.insert(
            dir.path().to_path_buf(),
            HashMap::from([(
                active_id,
                Preview {
                    path: active.clone(),
                    created: old,
                    busy: Arc::new(AtomicBool::new(true)),
                    plan: None,
                },
            )]),
        );
        cleanup_previews(dir.path(), &mut records);
        assert!(!orphan.exists());
        assert!(!plan.exists());
        assert!(note.exists());
        assert!(active.exists());
        // No in-memory registry after restart: filesystem TTL still works.
        records.clear();
        cleanup_previews(dir.path(), &mut records);
        assert!(!active.exists());
        assert!(note.exists());
    }
    #[tokio::test]
    async fn cancelled_http_inspection_reclaims_stage_but_keeps_real_worker_slot() {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("data");
        fs::create_dir_all(&data).unwrap();
        let config = ServerConfig {
            data_dir: data.clone(),
            ..Default::default()
        };
        let certificate = tls::ensure(&config, time::OffsetDateTime::now_utc()).unwrap();
        let database = DatabaseActor::open(data.join("catalog.sqlite")).unwrap();
        let state = AppState {
            tmdb_jobs: Arc::new(crate::tmdb_api::TmdbJobService::default()),
            playback: PlaybackService::default(),
            database,
            library: Arc::new(Mutex::new(LibrarySettings::load(&config).unwrap())),
            config: Arc::new(config),
            certificate: Arc::new(Mutex::new(certificate.schedule)),
            scan: Arc::new(Mutex::new(Default::default())),
        };
        let dir = import_dir(&state).unwrap();
        let gate = Arc::new(InspectionGate {
            entered: AtomicBool::new(false),
            released: Mutex::new(false),
            wake: std::sync::Condvar::new(),
        });
        INSPECTION_GATES
            .get_or_init(Default::default)
            .lock()
            .unwrap()
            .insert(dir.clone(), gate.clone());
        struct ReleaseOnDrop(Arc<InspectionGate>);
        impl Drop for ReleaseOnDrop {
            fn drop(&mut self) {
                *self.0.released.lock().unwrap() = true;
                self.0.wake.notify_all();
            }
        }
        let release = ReleaseOnDrop(gate.clone());
        let app = router(state);
        let export_app = app.clone();
        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/admin/imports")
            .header("host", "localhost:8443")
            .body(Body::from("invalid synthetic ZIP"))
            .unwrap();
        let task = tokio::spawn(app.oneshot(req));
        tokio::time::timeout(Duration::from_secs(3), async {
            while !gate.entered.load(Ordering::Acquire) {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
        assert!(previews().lock().unwrap().get(&dir).unwrap().is_empty());
        assert!(
            inspection_slot(&dir).is_err(),
            "A cancelled waiter must not release a still-running blocking inspection"
        );
        let refused_export = export_app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/admin/exports")
                    .header("host", "localhost:8443")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            refused_export.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "Export must not start while a cancelled HTTP inspection still owns the real heavy worker"
        );
        drop(release);
        INSPECTION_GATES
            .get_or_init(Default::default)
            .lock()
            .unwrap()
            .remove(&dir);
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                if let Ok(slot) = inspection_slot(&dir) {
                    drop(slot);
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(database_profile_count(&data.join("catalog.sqlite")), 0);
    }

    #[tokio::test]
    async fn export_route_streams_checked_portable_zip_without_secrets_or_paths() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let root = base.join("videos");
        let data = base.join("data");
        let posters = data.join("posters");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&posters).unwrap();
        fs::write(root.join("movie.mp4"), b"video-never-exported").unwrap();
        fs::write(
            posters.join("cover.png"),
            include_bytes!("../../../icons/icon.png"),
        )
        .unwrap();
        let config = ServerConfig {
            data_dir: data.clone(),
            video_root: Some(root.clone()),
            ..Default::default()
        };
        let cert = tls::ensure(&config, time::OffsetDateTime::now_utc()).unwrap();
        let database = DatabaseActor::open(data.join("catalog.sqlite")).unwrap();
        let roots = root.clone();
        let poster = posters.join("cover.png");
        let (profile, media) = database
            .call(move |store| {
                let p = store.create_profile("Archive")?;
                let mut m = chill_catalog::scanner::parse_file(&roots, &roots.join("movie.mp4"));
                m.poster = Some(poster.to_string_lossy().into());
                m.poster_custom = true;
                store.snapshot(vec![m])?;
                let media = store.catalog(&p.id)?.remove(0);
                store.save_preferences(
                    &p.id,
                    &chill_domain::Preferences {
                        audio_language: "fr".into(),
                        subtitle_language: "en".into(),
                        autoplay_seconds: Some(20),
                    },
                )?;
                store.set_series_speed(&p.id, "series", 1.25)?;
                store.set_rating(&p.id, &media.id, Some(4), "video")?;
                store.create_playlist_with_members(
                    &p.id,
                    "Liste",
                    std::slice::from_ref(&media.id),
                )?;
                Ok((p.id, media.id))
            })
            .await
            .unwrap();
        let state = crate::api::AppState {
            tmdb_jobs: Arc::new(crate::tmdb_api::TmdbJobService::default()),
            playback: PlaybackService::default(),
            database,
            library: Arc::new(Mutex::new(LibrarySettings::load(&config).unwrap())),
            config: Arc::new(config),
            certificate: Arc::new(Mutex::new(cert.schedule)),
            scan: Arc::new(Mutex::new(Default::default())),
        };
        let app = router(state);
        let request = Request::builder()
            .method("POST")
            .uri("/api/v1/admin/exports")
            .header("host", "localhost:8443")
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let started: serde_json::Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        let export_id = started["export_id"].as_str().unwrap();
        let export_deadline = std::time::Instant::now() + Duration::from_secs(10);
        let ready = loop {
            assert!(
                std::time::Instant::now() < export_deadline,
                "background export timeout"
            );
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("/api/v1/admin/exports/{export_id}/status"))
                        .header("host", "localhost:8443")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let status: serde_json::Value =
                serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                    .unwrap();
            if status["state"] != "building" {
                break status;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        };
        assert_eq!(ready["state"], "ready");
        let id = ready["export_id"].as_str().unwrap();
        let request = Request::builder()
            .uri(format!("/api/v1/admin/exports/{id}"))
            .header("host", "localhost:8443")
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let content_length = response
            .headers()
            .get(header::CONTENT_LENGTH)
            .unwrap()
            .to_str()
            .unwrap()
            .parse::<u64>()
            .unwrap();
        assert_eq!(content_length, ready["bytes"].as_u64().unwrap());
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let archive = bytes.clone();
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut manifest = String::new();
        zip.by_name("manifest.json")
            .unwrap()
            .read_to_string(&mut manifest)
            .unwrap();
        let manifest: serde_json::Value = serde_json::from_str(&manifest).unwrap();
        let mut document = String::new();
        zip.by_name("data.json")
            .unwrap()
            .read_to_string(&mut document)
            .unwrap();
        let document: serde_json::Value = serde_json::from_str(&document).unwrap();
        let poster_ref = document["media"][0]["data"]["poster"].as_str().unwrap();
        let mut poster = Vec::new();
        zip.by_name(poster_ref)
            .unwrap()
            .read_to_end(&mut poster)
            .unwrap();
        assert_eq!(poster, include_bytes!("../../../icons/icon.png"));
        let expected = manifest["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["reference"] == poster_ref)
            .unwrap()["sha256"]
            .as_str()
            .unwrap();
        assert_eq!(expected, format!("{:x}", Sha256::digest(&poster)));
        assert_eq!(document["profiles"][0]["id"], profile);
        assert_eq!(
            document["personal"][0]["preferences"]["audio_language"],
            "fr"
        );
        assert_eq!(
            document["personal"][0]["playlists"][0]["media_ids"][0],
            media
        );
        let rendered = document.to_string();
        assert!(
            !rendered.contains("video-never-exported")
                && !rendered.contains(root.to_string_lossy().as_ref())
                && !rendered.contains("tmdb_token")
        );
        assert_eq!(document["leases"], serde_json::json!([]));
        let before_profiles = database_profile_count(&data.join("catalog.sqlite"));
        let request = Request::builder()
            .method("POST")
            .uri("/api/v1/admin/imports")
            .header("host", "localhost:8443")
            .body(Body::from(archive))
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let preview: serde_json::Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(preview["export_id"], id);
        assert_eq!(preview["counts"]["media"], 1);
        assert_eq!(preview["profiles"][0]["id"], profile);
        assert_eq!(preview["roots"], serde_json::json!(["library"]));
        assert_eq!(preview["apply_supported"], true);
        assert_eq!(
            before_profiles,
            database_profile_count(&data.join("catalog.sqlite"))
        );
        let preview_id = preview["preview_id"].as_str().unwrap();
        let staged = data.join("imports").join(format!("{preview_id}.zip"));
        assert!(staged.exists());
        let plan_body = serde_json::json!({"profile_map":{profile.clone():{"kind":"new","name":"Imported archive"}},"root_prefix":"Imported","conflict_policy":"keep_destination"});
        let request = Request::builder()
            .method("POST")
            .uri(format!("/api/v1/admin/imports/{preview_id}/plan"))
            .header("host", "localhost:8443")
            .header("content-type", "application/json")
            .body(Body::from(plan_body.to_string()))
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let plan: serde_json::Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(plan["apply_supported"], true);
        assert_eq!(
            before_profiles,
            database_profile_count(&data.join("catalog.sqlite")),
            "Preparing a plan is read-only"
        );
        let plan_id = plan["plan_id"].as_str().unwrap();
        let apply_uri = format!("/api/v1/admin/imports/{preview_id}/apply");
        let apply_body = serde_json::json!({"plan_id":plan_id,"confirmed":true});
        for repeated in [false, true] {
            let request = Request::builder()
                .method("POST")
                .uri(&apply_uri)
                .header("host", "localhost:8443")
                .header("content-type", "application/json")
                .body(Body::from(apply_body.to_string()))
                .unwrap();
            let response = app.clone().oneshot(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let result: serde_json::Value =
                serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                    .unwrap();
            assert_eq!(result["already_imported"], repeated);
            assert_eq!(
                database_profile_count(&data.join("catalog.sqlite")),
                before_profiles + 1
            );
        }
        let imported = plan["profiles"][&profile].as_str().unwrap();
        let check = rusqlite::Connection::open(data.join("catalog.sqlite")).unwrap();
        let (language,rating,order):(String,i64,i64)=check.query_row("SELECT p.audio_language,s.rating,m.position FROM cc_preferences p JOIN cc_profile_state s ON s.profile_id=p.profile_id JOIN cc_playlists l ON l.profile_id=p.profile_id JOIN cc_playlist_members m ON m.playlist_id=l.id WHERE p.profile_id=?1",[imported],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
        assert_eq!((language, rating, order), ("fr".into(), 4, 0));
        let poster_path: String = check
            .query_row(
                "SELECT json_extract(data,'$.poster') FROM media WHERE id='Imported/movie.mp4'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            fs::read(poster_path).unwrap(),
            include_bytes!("../../../icons/icon.png")
        );
        assert_eq!(
            fs::read(root.join("movie.mp4")).unwrap(),
            b"video-never-exported"
        );
        let staged_plan = data.join("imports").join(format!("{plan_id}.plan"));
        assert!(staged_plan.is_file());
        let request = Request::builder()
            .method("DELETE")
            .uri(format!("/api/v1/admin/imports/{preview_id}"))
            .header("host", "localhost:8443")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            app.clone().oneshot(request).await.unwrap().status(),
            StatusCode::NO_CONTENT
        );
        assert!(!staged.exists());
        assert!(!staged_plan.exists());
        let request = Request::builder()
            .uri("/api/v1/admin/exports/not-a-uuid")
            .header("host", "localhost:8443")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            app.oneshot(request).await.unwrap().status(),
            StatusCode::NOT_FOUND
        );
    }

    fn database_profile_count(path: &std::path::Path) -> i64 {
        rusqlite::Connection::open(path)
            .unwrap()
            .query_row("SELECT COUNT(*) FROM cc_profiles", [], |row| row.get(0))
            .unwrap()
    }

    #[tokio::test]
    async fn archive_preview_refuses_traversal_unknown_versions_and_bad_checksums_without_database_writes()
     {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("data");
        fs::create_dir_all(&data).unwrap();
        let config = ServerConfig {
            data_dir: data.clone(),
            ..Default::default()
        };
        let cert = tls::ensure(&config, time::OffsetDateTime::now_utc()).unwrap();
        let database = DatabaseActor::open(data.join("catalog.sqlite")).unwrap();
        let state = crate::api::AppState {
            tmdb_jobs: Arc::new(crate::tmdb_api::TmdbJobService::default()),
            playback: PlaybackService::default(),
            database,
            library: Arc::new(Mutex::new(LibrarySettings::load(&config).unwrap())),
            config: Arc::new(config),
            certificate: Arc::new(Mutex::new(cert.schedule)),
            scan: Arc::new(Mutex::new(Default::default())),
        };
        let app = router(state);
        let before = database_profile_count(&data.join("catalog.sqlite"));
        for (name, manifest) in [
            (
                "../data.json",
                serde_json::json!({"format_version":1,"export_id":uuid::Uuid::new_v4(),"entries":[]}),
            ),
            (
                "data.json",
                serde_json::json!({"format_version":2,"export_id":uuid::Uuid::new_v4(),"entries":[]}),
            ),
        ] {
            let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
            let options = SimpleFileOptions::default();
            writer.start_file("manifest.json", options).unwrap();
            writer.write_all(manifest.to_string().as_bytes()).unwrap();
            writer.start_file(name, options).unwrap();
            writer.write_all(b"{}").unwrap();
            let body = writer.finish().unwrap().into_inner();
            let request = Request::builder()
                .method("POST")
                .uri("/api/v1/admin/imports")
                .header("host", "localhost:8443")
                .body(Body::from(body))
                .unwrap();
            assert!(
                app.clone()
                    .oneshot(request)
                    .await
                    .unwrap()
                    .status()
                    .is_client_error()
            );
        }
        let export_id = uuid::Uuid::new_v4().to_string();
        let document = serde_json::json!({"format_version":1,"export_id":export_id,"profiles":[],"media":[],"sagas":[],"personal":[],"leases":[]}).to_string();
        let manifest = serde_json::json!({"format_version":1,"export_id":export_id,"entries":[{"reference":"data.json","sha256":"00".repeat(32)}]}).to_string();
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default();
        writer.start_file("manifest.json", options).unwrap();
        writer.write_all(manifest.as_bytes()).unwrap();
        writer.start_file("data.json", options).unwrap();
        writer.write_all(document.as_bytes()).unwrap();
        let request = Request::builder()
            .method("POST")
            .uri("/api/v1/admin/imports")
            .header("host", "localhost:8443")
            .body(Body::from(writer.finish().unwrap().into_inner()))
            .unwrap();
        assert_eq!(
            app.clone().oneshot(request).await.unwrap().status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(before, database_profile_count(&data.join("catalog.sqlite")));
        assert!(fs::read_dir(data.join("imports")).unwrap().next().is_none());
    }
}

async fn download(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    if uuid::Uuid::parse_str(&id).is_err() {
        return Err(ApiError(StatusCode::NOT_FOUND, "EXPORT_NOT_FOUND"));
    }
    let path = export_dir(&state)?.join(format!("{id}.zip"));
    let file = tokio::fs::File::open(path)
        .await
        .map_err(|_| ApiError(StatusCode::NOT_FOUND, "EXPORT_NOT_FOUND"))?;
    let metadata = file
        .metadata()
        .await
        .map_err(|_| ApiError(StatusCode::NOT_FOUND, "EXPORT_NOT_FOUND"))?;
    if !metadata.is_file() || metadata.len() > MAX_COMPRESSED {
        return Err(ApiError(StatusCode::NOT_FOUND, "EXPORT_NOT_FOUND"));
    }
    let size = metadata.len();
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/zip")
        .header(header::CONTENT_LENGTH, size)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=chillcenter-{id}.zip"),
        )
        .header(header::CACHE_CONTROL, "no-store")
        .body(Body::from_stream(ReaderStream::with_capacity(
            file.take(size),
            64 * 1024,
        )))
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "EXPORT_STORAGE"))
}
