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
static UPLOAD_ACTIVE: AtomicBool = AtomicBool::new(false);

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

struct UploadGuard;
impl Drop for UploadGuard {
    fn drop(&mut self) {
        UPLOAD_ACTIVE.store(false, Ordering::Release);
    }
}
struct PartialFile(PathBuf);
impl Drop for PartialFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[derive(Serialize)]
struct ExportReady {
    export_id: String,
    bytes: u64,
    entries: u64,
    state: &'static str,
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
        .route("/api/v1/admin/exports", post(create))
        .route("/api/v1/admin/exports/{id}", get(download))
        // This endpoint writes the body directly to service-owned staging; it
        // intentionally replaces the router's normal 1 MiB JSON body limit.
        .route(
            "/api/v1/admin/imports",
            post(stage_and_preview).layer(DefaultBodyLimit::max(MAX_COMPRESSED as usize)),
        )
        .route("/api/v1/admin/imports/{id}/plan", post(prepare_plan))
        .route("/api/v1/admin/imports/{id}/apply", post(apply_plan))
        .route("/api/v1/admin/imports/{id}", delete(cancel_preview))
}

async fn prepare_plan(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<ImportMapping>,
) -> Result<Json<PlanReady>, ApiError> {
    let dir = import_dir(&state)?;
    let (path, previous, _guard) = reserve_preview(&dir, &id)?;
    let document = tokio::task::spawn_blocking(move || read_archive_document(&path))
        .await
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "IMPORT_INVALID"))??;
    let library = state.library.clone();
    let (plan, root_revision) = state
        .database
        .call(move |store| {
            let library = library.lock().map_err(|_| "REQUEST_TIMEOUT")?;
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
    let (archive, stored, _guard) = reserve_preview(&dir, &id)?;
    let stored = stored
        .filter(|p| p.id == request.plan_id)
        .ok_or(ApiError(StatusCode::CONFLICT, "IMPORT_PLAN_STALE"))?;
    let cache = state.config.data_dir.join("posters");
    let plan_path = stored.path;
    let (plan, mut posters) = tokio::task::spawn_blocking(move || {
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
    // Ownership follows the transaction, not the HTTP response: a cancelled
    // client must never delete artwork referenced by an already committed DB.
    let result = state
        .database
        .call(move |store| {
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
    Ok(path)
}

fn previews() -> &'static Mutex<HashMap<PathBuf, HashMap<String, Preview>>> {
    PREVIEWS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cleanup_previews(dir: &FsPath, records: &mut HashMap<PathBuf, HashMap<String, Preview>>) {
    let Some(previews) = records.get_mut(dir) else {
        return;
    };
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
}

fn staging_usage(dir: &FsPath) -> u64 {
    fs::read_dir(dir).ok().into_iter().flatten().filter_map(|entry| {
        let entry = entry.ok()?;
        let name = entry.file_name();
        let name = name.to_str()?;
        let (id, ext) = name.rsplit_once('.')?;
        (uuid::Uuid::parse_str(id).is_ok() && matches!(ext, "partial" | "zip" | "plan" | "tmp"))
            .then(|| entry.metadata().ok().map(|m| m.len())).flatten()
    }).sum()
}

async fn stage_and_preview(
    State(state): State<AppState>,
    body: Body,
) -> Result<Json<ImportPreview>, ApiError> {
    if UPLOAD_ACTIVE
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err(ApiError(
            StatusCode::TOO_MANY_REQUESTS,
            "IMPORT_UPLOAD_BUSY",
        ));
    }
    let _upload_guard = UploadGuard;
    let dir = import_dir(&state)?;
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
    }
    let id = uuid::Uuid::new_v4().to_string();
    let partial = dir.join(format!("{id}.partial"));
    let partial_guard = PartialFile(partial.clone());
    let staged = dir.join(format!("{id}.zip"));
    let mut options = tokio::fs::OpenOptions::new();
    options
        .write(true)
        .create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut output = options.open(&partial)
        .await
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
        if staging_usage(&dir).saturating_add(chunk.len() as u64) > MAX_STAGING_DISK {
            return Err(ApiError(StatusCode::INSUFFICIENT_STORAGE, "IMPORT_STAGING_FULL"));
        }
        tokio::io::AsyncWriteExt::write_all(&mut output, &chunk)
            .await
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "IMPORT_STORAGE"))?;
    }
    tokio::io::AsyncWriteExt::flush(&mut output)
        .await
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "IMPORT_STORAGE"))?;
    output
        .sync_all()
        .await
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "IMPORT_STORAGE"))?;
    drop(output);
    fs::rename(&partial, &staged)
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "IMPORT_STORAGE"))?;
    let staged_guard = PartialFile(staged.clone());
    let inspect_id = id.clone();
    let checked = tokio::task::spawn_blocking({
        let path = staged.clone();
        move || inspect_archive(&path, &inspect_id)
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
    std::mem::forget(partial_guard);
    std::mem::forget(staged_guard);
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
        || document.get("format_version").and_then(|v| v.as_u64()) != Some(1)
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

fn export_dir(state: &AppState) -> Result<PathBuf, ApiError> {
    let path = state.config.data_dir.join("exports");
    fs::create_dir_all(&path)
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
    Ok(path)
}

async fn create(State(state): State<AppState>) -> Result<Json<ExportReady>, ApiError> {
    let id = uuid::Uuid::new_v4().to_string();
    let dir = export_dir(&state)?;
    let partial = dir.join(format!("{id}.partial"));
    let ready = dir.join(format!("{id}.zip"));
    let data_stage = dir.join(format!("{id}.data.partial"));
    let poster_root = state.config.data_dir.join("posters");
    let writer_path = data_stage.clone();
    let export_id = id.clone();
    let writer_poster_root = poster_root.clone();
    let posters = state
        .database
        .call(move |store| {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&writer_path)
                .map_err(|_| "Export de staging inaccessible.".to_string())?;
            let posters = store.write_archive_json(&mut file, &export_id, &writer_poster_root)?;
            file.flush()
                .map_err(|_| "Export de staging inaccessible.")?;
            file.sync_all()
                .map_err(|_| "Export de staging inaccessible.")?;
            Ok(posters)
        })
        .await?;
    let data_size = fs::metadata(&data_stage)
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?
        .len();
    if data_size > MAX_DECOMPRESSED {
        let _ = fs::remove_file(&data_stage);
        return Err(ApiError(StatusCode::PAYLOAD_TOO_LARGE, "EXPORT_TOO_LARGE"));
    }
    let result = (|| -> Result<(u64, u64), ApiError> {
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&partial)
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        let mut checksums =
            vec![serde_json::json!({"reference":"data.json","sha256":hash_file(&data_stage)?})];
        let mut poster_hashes = BTreeMap::new();
        for poster in &posters {
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
        file.sync_all()
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
        let size = file
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
            fs::rename(&partial, &ready)
                .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
            fs::File::open(&dir)
                .and_then(|directory| directory.sync_all())
                .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
            let _ = fs::remove_file(&data_stage);
            Ok(Json(ExportReady {
                export_id: id,
                bytes,
                entries,
                state: "ready",
            }))
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
                store.create_playlist_with_members(&p.id, "Liste", &[media.id.clone()])?;
                Ok((p.id, media.id))
            })
            .await
            .unwrap();
        let state = crate::api::AppState {
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
        assert_eq!(response.status(), StatusCode::OK);
        let ready: serde_json::Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(ready["state"], "ready");
        let id = ready["export_id"].as_str().unwrap();
        let request = Request::builder()
            .uri(format!("/api/v1/admin/exports/{id}"))
            .header("host", "localhost:8443")
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
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
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/zip")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=chillcenter-{id}.zip"),
        )
        .header(header::CACHE_CONTROL, "no-store")
        .body(Body::from_stream(ReaderStream::new(file)))
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "EXPORT_STORAGE"))
}
