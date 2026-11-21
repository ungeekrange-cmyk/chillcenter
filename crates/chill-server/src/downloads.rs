//! Profile-bound, version-pinned downloads.  These transfers deliberately do
//! not acquire playback leases or write playback progress.
use crate::{
    api::{ApiError, AppState, Scheme},
    media_stream::{FileVersion, open_indexed, range},
};
use axum::{
    Json, Router,
    body::{Body, Bytes},
    extract::{Extension, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use chill_catalog::model::Media;
use chill_domain::{
    PersonalState, Preferences,
    store::{OfflineProgress, OfflineProgressApply},
};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fs::File,
    io::{self, Read},
    os::unix::fs::FileExt,
    path::PathBuf,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

const PAGE: usize = 20;
const MAX_SELECTED: usize = 2_000;
const MAX_SESSIONS: usize = 2;
const MAX_PER_DEVICE: usize = 1;
const SESSION_TTL: Duration = Duration::from_secs(90);
const CHUNK: usize = 64 * 1024;
const MAX_SUBTITLE: u64 = 16 * 1024 * 1024;
const MAX_SIDECARS: usize = 64;

#[derive(Clone)]
struct Sidecar {
    id: String,
    title: String,
    language: Option<String>,
    path: PathBuf,
    version: FileVersion,
}
#[derive(Clone)]
struct Session {
    profile: String,
    media: String,
    device: String,
    root: PathBuf,
    root_revision: u64,
    path: PathBuf,
    version: FileVersion,
    expires: Instant,
    sidecars: Vec<Sidecar>,
    cancelled: Arc<AtomicBool>,
}
#[derive(Default)]
struct Sessions {
    values: BTreeMap<String, Session>,
}
static SESSIONS: OnceLock<Mutex<Sessions>> = OnceLock::new();
static IO: OnceLock<Arc<Semaphore>> = OnceLock::new();
fn sessions() -> &'static Mutex<Sessions> {
    SESSIONS.get_or_init(|| Mutex::new(Sessions::default()))
}
fn io_slots() -> Arc<Semaphore> {
    IO.get_or_init(|| Arc::new(Semaphore::new(MAX_SESSIONS)))
        .clone()
}
fn prune(values: &mut Sessions) {
    let now = Instant::now();
    values.values.retain(|_, s| now < s.expires);
}
#[allow(dead_code)]
pub(crate) fn busy() -> bool {
    sessions()
        .lock()
        .map(|mut s| {
            prune(&mut s);
            !s.values.is_empty()
        })
        .unwrap_or(true)
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/profiles/{profile}/downloads/plan", get(plan))
        .route(
            "/profiles/{profile}/downloads/{media}/session",
            post(session).delete(cancel),
        )
        .route(
            "/profiles/{profile}/downloads/{media}/progress",
            put(progress),
        )
        .route(
            "/profiles/{profile}/downloads/{media}/content",
            get(content),
        )
        .route(
            "/profiles/{profile}/downloads/{media}/subtitles/{id}",
            get(subtitle),
        )
}
fn https(scheme: Scheme) -> Result<(), ApiError> {
    (scheme == Scheme::Https).then_some(()).ok_or(ApiError(
        StatusCode::UPGRADE_REQUIRED,
        "DOWNLOAD_HTTPS_REQUIRED",
    ))
}
fn one_header(headers: &HeaderMap, name: &str) -> Result<String, ApiError> {
    let all: Vec<_> = headers.get_all(name).iter().collect();
    if all.len() != 1 {
        return Err(ApiError(StatusCode::BAD_REQUEST, "DOWNLOAD_TOKEN_INVALID"));
    }
    let value = all[0]
        .to_str()
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "DOWNLOAD_TOKEN_INVALID"))?;
    if value.is_empty() || value.len() > 128 {
        return Err(ApiError(StatusCode::BAD_REQUEST, "DOWNLOAD_TOKEN_INVALID"));
    }
    Ok(value.into())
}
fn token_header(headers: &HeaderMap) -> Result<String, ApiError> {
    let token = one_header(headers, "x-chill-download")?;
    if !token
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Err(ApiError(StatusCode::BAD_REQUEST, "DOWNLOAD_TOKEN_INVALID"));
    }
    Ok(token)
}
fn client_media(media: &Media) -> serde_json::Value {
    serde_json::json!({"id":media.id,"title":media.title,"category":media.category,"year":media.year,"series":media.series,"season":media.season,"episode":media.episode,"genres":media.genres,"overview":media.overview,"poster":media.poster,"runtime_minutes":media.runtime_minutes,"cast":media.cast,"directors":media.directors,"series_overview":media.series_overview,"watched":media.watched,"available":media.available,"added_at":media.added_at,"tmdb_id":media.tmdb_id})
}
fn series_key(media: &Media, anchor: &str) -> Option<String> {
    (media.category == "serie").then(|| {
        media
            .tmdb_id
            .map(|id| format!("tmdb:tv:{id}"))
            .or_else(|| {
                media
                    .series
                    .as_ref()
                    .map(|series| format!("series:{series}"))
            })
            .unwrap_or_else(|| format!("media:{anchor}"))
    })
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanQuery {
    media_id: String,
    scope: String,
    offset: Option<usize>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SessionRequest {
    version: String,
    device_id: String,
}

async fn open_version(root: PathBuf, path: PathBuf) -> Result<(File, FileVersion), ApiError> {
    let permit = io_slots()
        .try_acquire_owned()
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "DOWNLOAD_BUSY"))?;
    tokio::time::timeout(
        Duration::from_secs(5),
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            open_indexed(&root, &path)
        }),
    )
    .await
    .map_err(|_| ApiError(StatusCode::GATEWAY_TIMEOUT, "DOWNLOAD_TIMEOUT"))?
    .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "DOWNLOAD_UNAVAILABLE"))?
    .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "DOWNLOAD_UNAVAILABLE"))
}
async fn inspect_sidecars(root: PathBuf, path: PathBuf) -> Result<Vec<Sidecar>, ApiError> {
    let permit = io_slots()
        .try_acquire_owned()
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "DOWNLOAD_BUSY"))?;
    tokio::time::timeout(
        Duration::from_secs(5),
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            sidecars(&root, &path)
        }),
    )
    .await
    .map_err(|_| ApiError(StatusCode::GATEWAY_TIMEOUT, "DOWNLOAD_TIMEOUT"))?
    .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "DOWNLOAD_UNAVAILABLE"))?
    .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "DOWNLOAD_UNAVAILABLE"))
}
async fn source(
    state: &AppState,
    profile: String,
    media: String,
) -> Result<(Media, PathBuf, PathBuf, u64), ApiError> {
    let (item, path) = state
        .database
        .call(move |store| Ok((store.media(&profile, &media)?, store.media_path(&media)?)))
        .await?;
    if !item.available {
        return Err(ApiError(StatusCode::NOT_FOUND, "DOWNLOAD_UNAVAILABLE"));
    }
    let settings = state
        .library
        .lock()
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "DOWNLOAD_BUSY"))?
        .clone();
    let root = settings.root_for_path(&path).ok_or(ApiError(
        StatusCode::PRECONDITION_FAILED,
        "DOWNLOAD_CHANGED",
    ))?;
    Ok((item, root, path, settings.revision))
}
fn sidecars(root: &PathBuf, video: &PathBuf) -> io::Result<Vec<Sidecar>> {
    let stem = video
        .file_stem()
        .and_then(|v| v.to_str())
        .ok_or(io::ErrorKind::InvalidInput)?;
    let parent = video.parent().ok_or(io::ErrorKind::InvalidInput)?;
    let mut found = Vec::new();
    for entry in std::fs::read_dir(parent)?.take(16_384) {
        let path = entry?.path();
        let Some(name) = path.file_name().and_then(|v| v.to_str()) else {
            continue;
        };
        let lower = name.to_ascii_lowercase();
        if ![".srt", ".vtt", ".ass", ".ssa"]
            .iter()
            .any(|ext| lower.ends_with(ext))
            || !(name == format!("{stem}.srt") || name.starts_with(&format!("{stem}.")))
        {
            continue;
        }
        let (_, version) = open_indexed(root, &path)?;
        if version.size > MAX_SUBTITLE {
            continue;
        }
        let language = name
            .strip_prefix(stem)
            .and_then(|s| s.strip_prefix('.'))
            .and_then(|s| s.split('.').next())
            .filter(|s| (2..=16).contains(&s.len()))
            .map(|s| s.to_ascii_lowercase());
        if found.len() >= MAX_SIDECARS {
            break;
        }
        found.push(Sidecar {
            id: uuid::Uuid::new_v4().to_string(),
            title: name.into(),
            language,
            path,
            version,
        });
    }
    found.sort_by(|a, b| a.title.cmp(&b.title));
    Ok(found)
}
async fn plan(
    Extension(scheme): Extension<Scheme>,
    State(state): State<AppState>,
    Path(profile): Path<String>,
    Query(query): Query<PlanQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    https(scheme)?;
    if query.media_id.len() > 128 || !matches!(query.scope.as_str(), "video" | "season" | "series")
    {
        return Err(ApiError(StatusCode::BAD_REQUEST, "DOWNLOAD_PLAN_INVALID"));
    }
    let anchor = query.media_id.clone();
    let scope = query.scope.clone();
    let p = profile.clone();
    let selected = state
        .database
        .call(move |store| {
            let current = store.media(&p, &anchor)?;
            let mut values = match scope.as_str() {
                "video" => vec![current],
                "season" => store
                    .series(&p, &format!("media:{}", current.id))?
                    .into_iter()
                    .filter(|m| m.season == current.season)
                    .collect(),
                _ => store.series(&p, &format!("media:{}", current.id))?,
            };
            values.retain(|m| m.available);
            values.truncate(MAX_SELECTED);
            Ok(values)
        })
        .await?;
    let offset = query.offset.unwrap_or(0).min(selected.len());
    let mut items = Vec::new();
    let mut skipped = 0usize;
    let mut index = offset;
    while index < selected.len() && items.len() < PAGE {
        let media = selected[index].clone();
        index += 1;
        let (_, root, path, revision) =
            match source(&state, profile.clone(), media.id.clone()).await {
                Ok(v) => v,
                Err(_) => {
                    skipped += 1;
                    continue;
                }
            };
        let (_, version) = match open_version(root, path).await {
            Ok(v) => v,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        items.push(serde_json::json!({"media":client_media(&media),"series_key":series_key(&media, &query.media_id),"bytes":version.size,"version":version.etag(revision)}));
    }
    Ok(Json(
        serde_json::json!({"items":items,"skipped":skipped,"next_offset":(index < selected.len()).then_some(index)}),
    ))
}
async fn session(
    Extension(scheme): Extension<Scheme>,
    State(state): State<AppState>,
    Path((profile, media)): Path<(String, String)>,
    Json(body): Json<SessionRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    https(scheme)?;
    if uuid::Uuid::parse_str(&body.device_id).is_err() || body.version.len() > 128 {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "DOWNLOAD_SESSION_INVALID",
        ));
    }
    let p = profile.clone();
    let m = media.clone();
    let (personal, preferences, progress_revision) = state
        .database
        .call(move |store| {
            Ok((
                store.personal_state(&p, &m)?,
                store.preferences(&p)?,
                store.offline_progress_revision(&p, &m)?,
            ))
        })
        .await?;
    let (_, root, path, revision) = source(&state, profile.clone(), media.clone()).await?;
    let (_, version) = open_version(root.clone(), path.clone()).await?;
    if body.version != version.etag(revision) {
        return Err(ApiError(
            StatusCode::PRECONDITION_FAILED,
            "DOWNLOAD_CHANGED",
        ));
    }
    let sidecars = inspect_sidecars(root.clone(), path.clone()).await?;
    let token = uuid::Uuid::new_v4().to_string();
    {
        let mut all = sessions()
            .lock()
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "DOWNLOAD_BUSY"))?;
        prune(&mut all);
        if all.values.len() >= MAX_SESSIONS
            || all
                .values
                .values()
                .filter(|s| s.device == body.device_id)
                .count()
                >= MAX_PER_DEVICE
        {
            return Err(ApiError(StatusCode::SERVICE_UNAVAILABLE, "DOWNLOAD_BUSY"));
        }
        all.values.insert(
            token.clone(),
            Session {
                profile: profile.clone(),
                media: media.clone(),
                device: body.device_id,
                root,
                root_revision: revision,
                path,
                version: version.clone(),
                expires: Instant::now() + SESSION_TTL,
                sidecars: sidecars.clone(),
                cancelled: Arc::new(AtomicBool::new(false)),
            },
        );
    }
    Ok(Json(session_reply(
        &token,
        &version,
        revision,
        &sidecars,
        preferences,
        personal,
        progress_revision,
    )))
}
fn session_reply(
    token: &str,
    version: &FileVersion,
    revision: u64,
    sidecars: &[Sidecar],
    preferences: Preferences,
    personal: PersonalState,
    progress_revision: String,
) -> serde_json::Value {
    serde_json::json!({"token":token,"bytes":version.size,"version":version.etag(revision),"subtitles":sidecars.iter().map(|s| serde_json::json!({"id":s.id,"title":s.title,"language":s.language,"bytes":s.version.size})).collect::<Vec<_>>(),"preferences":preferences,"personal":personal,"progress_revision":progress_revision})
}
fn session_for(headers: &HeaderMap, profile: &str, media: &str) -> Result<Session, ApiError> {
    let token = token_header(headers)?;
    let mut all = sessions()
        .lock()
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "DOWNLOAD_BUSY"))?;
    prune(&mut all);
    all.values
        .get(&token)
        .filter(|s| s.profile == profile && s.media == media)
        .cloned()
        .ok_or(ApiError(StatusCode::CONFLICT, "DOWNLOAD_SESSION_INVALID"))
}
async fn content(
    Extension(scheme): Extension<Scheme>,
    State(_state): State<AppState>,
    Path((profile, media)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    https(scheme)?;
    let session = session_for(&headers, &profile, &media)?;
    let (file, actual) = open_version(session.root.clone(), session.path.clone()).await?;
    if actual != session.version {
        return Err(ApiError(
            StatusCode::PRECONDITION_FAILED,
            "DOWNLOAD_CHANGED",
        ));
    }
    if headers.contains_key("if-match")
        && one_header(&headers, "if-match")? != actual.etag(session.root_revision)
    {
        return Err(ApiError(
            StatusCode::PRECONDITION_FAILED,
            "DOWNLOAD_CHANGED",
        ));
    }
    let requested = one_header(&headers, "range")?;
    let Some((start, end)) = range(&requested, actual.size) else {
        let mut r = ApiError(StatusCode::RANGE_NOT_SATISFIABLE, "RANGE_INVALID").into_response();
        r.headers_mut().insert(
            "content-range",
            format!("bytes */{}", actual.size).parse().unwrap(),
        );
        return Ok(r);
    };
    let length = end - start + 1;
    let permit = io_slots()
        .try_acquire_owned()
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "DOWNLOAD_BUSY"))?;
    Ok(range_response(
        file,
        actual,
        start,
        length,
        session.root_revision,
        session.cancelled,
        permit,
    ))
}
fn range_response(
    file: File,
    expected: FileVersion,
    start: u64,
    length: u64,
    revision: u64,
    cancelled: Arc<AtomicBool>,
    permit: OwnedSemaphorePermit,
) -> Response {
    let file = Arc::new(file);
    let total = expected.size;
    let etag = expected.etag(revision);
    let stream_expected = expected.clone();
    let stream = futures_util::stream::unfold(
        (file, start, length, stream_expected, cancelled, permit),
        |(file, offset, remaining, expected, cancelled, permit)| async move {
            if remaining == 0 {
                return None;
            }
            if cancelled.load(Ordering::Acquire) {
                return Some((
                    Err(io::Error::from(io::ErrorKind::Interrupted)),
                    (file, offset, 0, expected, cancelled, permit),
                ));
            }
            let n = remaining.min(CHUNK as u64) as usize;
            let file2 = file.clone();
            let expected2 = expected.clone();
            let result = tokio::task::spawn_blocking(move || {
                if FileVersion::from_metadata(&file2.metadata()?)? != expected2 {
                    return Err(io::ErrorKind::InvalidData.into());
                }
                let mut bytes = vec![0; n];
                let got = file2.read_at(&mut bytes, offset)?;
                if got == 0 {
                    return Err(io::ErrorKind::UnexpectedEof.into());
                }
                bytes.truncate(got);
                Ok::<_, io::Error>(bytes)
            })
            .await
            .map_err(|_| io::Error::from(io::ErrorKind::Other))
            .and_then(|v| v);
            match result {
                Ok(bytes) => {
                    let got = bytes.len() as u64;
                    Some((
                        Ok::<_, io::Error>(Bytes::from(bytes)),
                        (
                            file,
                            offset + got,
                            remaining - got,
                            expected,
                            cancelled,
                            permit,
                        ),
                    ))
                }
                Err(e) => Some((Err(e), (file, offset, 0, expected, cancelled, permit))),
            }
        },
    );
    Response::builder()
        .status(StatusCode::PARTIAL_CONTENT)
        .header("content-type", "application/octet-stream")
        .header("accept-ranges", "bytes")
        .header(
            "content-range",
            format!("bytes {start}-{}/{}", start + length - 1, total),
        )
        .header("content-length", length.to_string())
        .header("etag", etag)
        .header("cache-control", "no-store")
        .body(Body::from_stream(stream))
        .unwrap()
}
async fn subtitle(
    Extension(scheme): Extension<Scheme>,
    State(_state): State<AppState>,
    Path((profile, media, id)): Path<(String, String, String)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    https(scheme)?;
    let session = session_for(&headers, &profile, &media)?;
    let sidecar = session
        .sidecars
        .into_iter()
        .find(|s| s.id == id)
        .ok_or(ApiError(StatusCode::NOT_FOUND, "DOWNLOAD_SUBTITLE_INVALID"))?;
    let (file, actual) = open_version(session.root, sidecar.path).await?;
    if actual != sidecar.version {
        return Err(ApiError(
            StatusCode::PRECONDITION_FAILED,
            "DOWNLOAD_CHANGED",
        ));
    }
    let mut bytes = Vec::with_capacity(actual.size as usize);
    file.take(MAX_SUBTITLE + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "DOWNLOAD_UNAVAILABLE"))?;
    if bytes.len() as u64 > MAX_SUBTITLE || std::str::from_utf8(&bytes).is_err() {
        return Err(ApiError(
            StatusCode::UNPROCESSABLE_ENTITY,
            "DOWNLOAD_SUBTITLE_INVALID",
        ));
    }
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/plain; charset=utf-8")
        .header("content-length", bytes.len())
        .header("etag", actual.etag(session.root_revision))
        .header("cache-control", "no-store")
        .header("x-content-type-options", "nosniff")
        .body(Body::from(bytes))
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "DOWNLOAD_UNAVAILABLE"))
}

async fn progress(
    Extension(scheme): Extension<Scheme>,
    State(state): State<AppState>,
    Path((profile, media)): Path<(String, String)>,
    Json(body): Json<OfflineProgress>,
) -> Result<Json<serde_json::Value>, ApiError> {
    https(scheme)?;
    let playback = state.playback.clone();
    let p = profile.clone();
    let m = media.clone();
    let result = state
        .database
        .call(move |store| {
            if playback.profile_lease_active(&p) {
                return Ok(OfflineProgressApply::Conflict);
            }
            // `apply_offline_progress` deliberately does not call the lease credit
            // path: the client only declares its local state here.
            store.apply_offline_progress(&p, &m, &body)
        })
        .await?;
    match result {
        OfflineProgressApply::Applied { revision } => {
            Ok(Json(serde_json::json!({"revision":revision})))
        }
        OfflineProgressApply::Conflict => {
            Err(ApiError(StatusCode::CONFLICT, "DOWNLOAD_PROGRESS_CONFLICT"))
        }
    }
}
async fn cancel(
    Extension(scheme): Extension<Scheme>,
    Path((profile, media)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    https(scheme)?;
    let token = token_header(&headers)?;
    let mut all = sessions()
        .lock()
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "DOWNLOAD_BUSY"))?;
    prune(&mut all);
    if let Some(session) = all
        .values
        .get(&token)
        .filter(|s| s.profile == profile && s.media == media)
    {
        session.cancelled.store(true, Ordering::Release);
        all.values.remove(&token);
    }
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        api::{DatabaseActor, router},
        config::{LibrarySettings, ServerConfig},
    };
    use axum::{Router, body::Body, http::Request};
    use http_body_util::BodyExt;
    use serde_json::Value;
    use std::sync::{Arc, Mutex};
    use tower::ServiceExt;

    struct Fixture {
        _dir: tempfile::TempDir,
        app: Router,
        profile: String,
        other_profile: String,
        media: String,
        bytes: Vec<u8>,
    }
    async fn fixture() -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().canonicalize().unwrap();
        let root = base.join("videos");
        std::fs::create_dir(&root).unwrap();
        let bytes: Vec<_> = (0..300_000).map(|n| (n % 251) as u8).collect();
        std::fs::write(root.join("Film.mp4"), &bytes).unwrap();
        let config = ServerConfig {
            data_dir: base.join("data"),
            video_root: Some(root.clone()),
            ..Default::default()
        };
        std::fs::create_dir(&config.data_dir).unwrap();
        let cert = crate::tls::ensure(&config, time::OffsetDateTime::now_utc()).unwrap();
        let database = DatabaseActor::open(config.data_dir.join("catalog.sqlite")).unwrap();
        let scan_root = root.clone();
        let (profile, other_profile, media) = database
            .call(move |store| {
                let profile = store.create_profile("Download A")?;
                let other = store.create_profile("Download B")?;
                store.snapshot(vec![chill_catalog::scanner::parse_file(
                    &scan_root,
                    &scan_root.join("Film.mp4"),
                )])?;
                let media = store.catalog(&profile.id)?[0].id.clone();
                Ok((profile.id, other.id, media))
            })
            .await
            .unwrap();
        let playback = crate::playback::PlaybackService::default();
        let state = AppState {
            tmdb_jobs: Arc::new(crate::tmdb_api::TmdbJobService::default()),
            playback,
            database,
            library: Arc::new(Mutex::new(LibrarySettings::load(&config).unwrap())),
            config: Arc::new(config),
            certificate: Arc::new(Mutex::new(cert.schedule)),
            scan: Arc::new(Mutex::new(Default::default())),
        };
        Fixture {
            _dir: dir,
            app: router(state),
            profile,
            other_profile,
            media,
            bytes,
        }
    }
    async fn request(
        app: &Router,
        method: &str,
        uri: String,
        body: Value,
        headers: &[(&str, &str)],
    ) -> (StatusCode, Value, axum::http::HeaderMap) {
        let mut builder = Request::builder()
            .method(method)
            .uri(uri)
            .header("host", "localhost:8443");
        for (key, value) in headers {
            builder = builder.header(*key, *value);
        }
        let request = builder
            .header("content-type", "application/json")
            .body(Body::from(if body.is_null() {
                Vec::new()
            } else {
                serde_json::to_vec(&body).unwrap()
            }))
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let value = serde_json::from_slice(&body).unwrap_or(Value::Null);
        (status, value, headers)
    }

    #[tokio::test]
    async fn plan_session_range_cancel_and_offline_progress_are_profile_bound() {
        let f = fixture().await;
        let base = format!("/api/2026-10-14/profiles/{}/downloads", f.profile);
        let (status, plan, _) = request(
            &f.app,
            "GET",
            format!("{base}/plan?media_id={}&scope=video", f.media),
            Value::Null,
            &[],
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let item = &plan["items"][0];
        assert!(item["media"].get("path").is_none());
        let version = item["version"].as_str().unwrap().to_owned();
        let (status, session, _) = request(
            &f.app,
            "POST",
            format!("{base}/{}/session", f.media),
            serde_json::json!({"version":version,"device_id":uuid::Uuid::new_v4()}),
            &[],
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(session["progress_revision"], "0");
        let token = session["token"].as_str().unwrap().to_owned();
        let builder = Request::builder()
            .method("GET")
            .uri(format!("{base}/{}/content", f.media))
            .header("host", "localhost:8443")
            .header("x-chill-download", &token)
            .header("range", "bytes=11-21");
        let response = f
            .app
            .clone()
            .oneshot(builder.body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(status, StatusCode::PARTIAL_CONTENT);
        assert_eq!(headers["content-range"], "bytes 11-21/300000");
        assert_eq!(body.as_ref(), &f.bytes[11..22]);
        let (status, _, _) = request(
            &f.app,
            "GET",
            format!("{base}/{}/content", f.media),
            Value::Null,
            &[("x-chill-download", &token), ("range", "bytes=0-1,3-4")],
        )
        .await;
        assert_eq!(status, StatusCode::RANGE_NOT_SATISFIABLE);
        let other = format!(
            "/api/2026-10-14/profiles/{}/downloads/{}/content",
            f.other_profile, f.media
        );
        assert_eq!(
            request(
                &f.app,
                "GET",
                other,
                Value::Null,
                &[("x-chill-download", &token), ("range", "bytes=0-1")]
            )
            .await
            .0,
            StatusCode::CONFLICT
        );

        let event = uuid::Uuid::new_v4().to_string();
        let progress = serde_json::json!({"event_id":event,"base_revision":"0","position_ms":100,"duration_ms":1000,"watched":false});
        let progress_path = format!("{base}/{}/progress", f.media);
        let (status, first, _) =
            request(&f.app, "PUT", progress_path.clone(), progress.clone(), &[]).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(first["revision"], "1");
        assert_eq!(
            request(&f.app, "PUT", progress_path.clone(), progress, &[])
                .await
                .0,
            StatusCode::OK
        );

        let acquire = format!("/api/2026-10-14/profiles/{}/playback/acquire", f.profile);
        assert_eq!(request(&f.app, "POST", acquire, serde_json::json!({"media_id":f.media,"device_id":uuid::Uuid::new_v4(),"context":{"kind":"standalone","id":null}}), &[]).await.0, StatusCode::OK);
        assert_eq!(request(&f.app, "PUT", progress_path, serde_json::json!({"event_id":uuid::Uuid::new_v4(),"base_revision":"1","position_ms":200,"duration_ms":1000,"watched":false}), &[]).await.0, StatusCode::CONFLICT);

        assert_eq!(
            request(
                &f.app,
                "DELETE",
                format!("{base}/{}/session", f.media),
                Value::Null,
                &[("x-chill-download", &token)]
            )
            .await
            .0,
            StatusCode::NO_CONTENT
        );
        assert_eq!(
            request(
                &f.app,
                "GET",
                format!("{base}/{}/content", f.media),
                Value::Null,
                &[("x-chill-download", &token), ("range", "bytes=0-1")]
            )
            .await
            .0,
            StatusCode::CONFLICT
        );
        assert_eq!(f.bytes[11], 11);
    }
}
