use crate::api::{ApiError, AppState};
use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
};
use chill_catalog::model::{MediaPatch, SeriesPatch};
use chill_domain::{PlaybackContext, catalog_browser::BrowserQuery};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::Read;
use std::{
    process::Stdio,
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, Instant},
};
use uuid::Uuid;

const SYNC_PAGE: usize = 100;
const SYNC_MAX_SNAPSHOTS: usize = 3;
const SYNC_TTL: Duration = Duration::from_secs(120);
#[derive(Clone)]
struct StoredSnapshot {
    server: String,
    profile: String,
    revision: String,
    created: Instant,
    snapshot: Arc<chill_domain::SyncSnapshot>,
    complete: bool,
}
#[derive(Default)]
struct SnapshotCache {
    snapshots: HashMap<String, StoredSnapshot>,
}
static SNAPSHOTS: OnceLock<Mutex<SnapshotCache>> = OnceLock::new();
static SNAPSHOT_BOOT: OnceLock<String> = OnceLock::new();
fn snapshot_cache() -> &'static Mutex<SnapshotCache> {
    SNAPSHOTS.get_or_init(|| Mutex::new(SnapshotCache::default()))
}
fn snapshot_revision(server: &str, profile: &str, epoch: (u64, u64)) -> String {
    format!(
        "{server}:{}:{profile}:{}:{}",
        SNAPSHOT_BOOT.get_or_init(|| Uuid::new_v4().to_string()),
        epoch.0,
        epoch.1
    )
}

#[derive(Clone, Default, Serialize)]
pub struct ScanStatus {
    pub running: bool,
    pub last_error: Option<String>,
    pub count: Option<usize>,
}
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/admin/scan", get(scan_status).post(start_scan))
        .route("/profiles/{profile}/catalog", get(catalog))
        .route(
            "/profiles/{profile}/catalog/snapshot",
            post(create_snapshot),
        )
        .route(
            "/profiles/{profile}/catalog/snapshot/{snapshot}",
            get(snapshot_page),
        )
        .route("/sagas/{saga}/artwork", get(common_saga_artwork))
        .route(
            "/admin/unavailable",
            get(unavailable).post(forget_unavailable),
        )
        .route("/profiles/{profile}/media/{media}/related", get(related))
        .route(
            "/profiles/{profile}/media/{media}",
            get(detail).put(update_media),
        )
        .route("/profiles/{profile}/media/{media}/artwork", get(artwork))
        .route(
            "/profiles/{profile}/media/{media}/poster",
            put(upload_poster).layer(DefaultBodyLimit::max(8 * 1024 * 1024)),
        )
        .route(
            "/profiles/{profile}/media/{media}/series/poster",
            put(upload_series_poster).layer(DefaultBodyLimit::max(8 * 1024 * 1024)),
        )
        .route(
            "/profiles/{profile}/media/{media}/season/poster",
            put(upload_season_poster).layer(DefaultBodyLimit::max(8 * 1024 * 1024)),
        )
        .route(
            "/profiles/{profile}/media/{media}/subtitles",
            get(crate::media_stream::subtitles),
        )
        .route(
            "/profiles/{profile}/media/{media}/subtitles/{id}",
            get(crate::media_stream::subtitle),
        )
        .route(
            "/profiles/{profile}/series/{series}",
            get(series).put(edit_series),
        )
        .route(
            "/profiles/{profile}/series/{tv}/seasons/{season}/artwork",
            get(season_artwork),
        )
        .route(
            "/profiles/{profile}/sagas/{saga}/artwork",
            get(saga_artwork),
        )
        .route(
            "/profiles/{profile}/sagas/{saga}/poster",
            put(upload_saga_poster).layer(DefaultBodyLimit::max(8 * 1024 * 1024)),
        )
        .route(
            "/profiles/{profile}/sagas/{saga}/poster/member",
            put(saga_member_poster),
        )
        .route("/profiles/{profile}/sagas", get(sagas).post(create_saga))
        .route(
            "/profiles/{profile}/sagas/{saga}",
            get(saga).put(rename_saga).delete(delete_saga),
        )
        .route(
            "/profiles/{profile}/sagas/{saga}/members",
            post(add_saga_members).delete(remove_saga_members),
        )
        .route("/profiles/{profile}/sagas/{saga}/order", put(order_saga))
        .route(
            "/profiles/{profile}/playlists",
            get(playlists).post(create_playlist),
        )
        .route(
            "/profiles/{profile}/playlists/{playlist}",
            get(playlist).put(rename_playlist).delete(delete_playlist),
        )
        .route(
            "/profiles/{profile}/playlists/{playlist}/artwork",
            get(playlist_artwork),
        )
        .route(
            "/profiles/{profile}/playlists/{playlist}/poster",
            put(upload_playlist_poster).layer(DefaultBodyLimit::max(8 * 1024 * 1024)),
        )
        .route(
            "/profiles/{profile}/playlists/{playlist}/members",
            post(add_members),
        )
        .route(
            "/profiles/{profile}/playlists/{playlist}/members/{media}",
            delete(remove_member),
        )
        .route(
            "/profiles/{profile}/playlists/{playlist}/order",
            put(order_playlist),
        )
        .route("/profiles/{profile}/playback/next", post(next_video))
        .route(
            "/profiles/{profile}/media/{media}/playback-info",
            get(playback_info),
        )
        .route("/profiles/{profile}/media/{media}/watched", put(watched))
        .route("/profiles/{profile}/media/{media}/rating", put(rating))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotRequest {
    known_revision: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotPageQuery {
    offset: Option<usize>,
}
async fn create_snapshot(
    State(state): State<AppState>,
    Path(profile): Path<String>,
    Json(body): Json<SnapshotRequest>,
) -> Result<Json<Value>, ApiError> {
    let server = state
        .certificate
        .lock()
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "SYNC_BUSY"))?
        .server_id
        .clone();
    let epoch = state
        .database
        .call({
            let profile = profile.clone();
            move |s| s.sync_revision(&profile)
        })
        .await?;
    let revision = snapshot_revision(&server, &profile, epoch);
    if body.known_revision.as_deref() == Some(&revision) {
        return Ok(Json(json!({"revision":revision,"unchanged":true})));
    }
    let (captured_epoch, snapshot) = state
        .database
        .call({
            let profile = profile.clone();
            move |s| {
                let snapshot = s.sync_snapshot(&profile)?;
                Ok((snapshot.captured_revision, snapshot))
            }
        })
        .await?;
    let revision = snapshot_revision(&server, &profile, captured_epoch);
    if captured_epoch != epoch {
        return Err(ApiError(StatusCode::CONFLICT, "SYNC_SNAPSHOT_CHANGED"));
    }
    let id = Uuid::new_v4().to_string();
    let total = snapshot.items.len();
    let first = snapshot.items[..total.min(SYNC_PAGE)].to_vec();
    let sagas = snapshot.sagas.clone();
    let playlists = snapshot.playlists.clone();
    let series_revisions = snapshot.series_revisions.clone();
    let season_metadata = snapshot.season_metadata.clone();
    let has_more = total > first.len();
    {
        let mut cache = snapshot_cache()
            .lock()
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "SYNC_BUSY"))?;
        cache
            .snapshots
            .retain(|_, value| value.created.elapsed() < SYNC_TTL);
        if cache.snapshots.len() >= SYNC_MAX_SNAPSHOTS {
            let complete = cache
                .snapshots
                .iter()
                .filter(|(_, v)| v.complete)
                .min_by_key(|(_, v)| v.created)
                .map(|(k, _)| k.clone());
            if let Some(id) = complete {
                cache.snapshots.remove(&id);
            }
        }
        if cache.snapshots.len() >= SYNC_MAX_SNAPSHOTS
            || cache
                .snapshots
                .values()
                .map(|v| v.snapshot.serialized_bytes)
                .sum::<usize>()
                .saturating_add(snapshot.serialized_bytes)
                > 32 * 1024 * 1024
        {
            return Err(ApiError(StatusCode::TOO_MANY_REQUESTS, "SYNC_BUSY"));
        }
        let snapshot = Arc::new(snapshot);
        cache.snapshots.insert(
            id.clone(),
            StoredSnapshot {
                server,
                profile,
                revision: revision.clone(),
                created: Instant::now(),
                snapshot: Arc::clone(&snapshot),
                complete: !has_more,
            },
        );
    }
    let expiry_id = id.clone();
    tokio::spawn(async move {
        tokio::time::sleep(SYNC_TTL).await;
        if let Ok(mut cache) = snapshot_cache().lock() {
            cache.snapshots.remove(&expiry_id);
        }
    });
    Ok(Json(
        json!({"revision":revision,"snapshot_id":id,"total":total,"items":first,"sagas":sagas,"playlists":playlists,"series_revisions":series_revisions,"season_metadata":season_metadata,"has_more":has_more,"next_offset":if has_more { Some(SYNC_PAGE) } else { None::<usize> }}),
    ))
}
async fn snapshot_page(
    State(state): State<AppState>,
    Path((profile, snapshot)): Path<(String, String)>,
    Query(query): Query<SnapshotPageQuery>,
) -> Result<Json<Value>, ApiError> {
    let offset = query.offset.unwrap_or(0);
    let server = state
        .certificate
        .lock()
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "SYNC_BUSY"))?
        .server_id
        .clone();
    state
        .database
        .call({
            let profile = profile.clone();
            move |s| s.sync_revision(&profile).map(|_| ())
        })
        .await?;
    let cached = {
        let mut cache = snapshot_cache()
            .lock()
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "SYNC_BUSY"))?;
        cache
            .snapshots
            .retain(|_, value| value.created.elapsed() < SYNC_TTL);
        cache.snapshots.get(&snapshot).cloned()
    }
    .ok_or(ApiError(StatusCode::GONE, "SYNC_SNAPSHOT_EXPIRED"))?;
    if cached.server != server || cached.profile != profile || offset > cached.snapshot.items.len()
    {
        return Err(ApiError(StatusCode::NOT_FOUND, "SYNC_SNAPSHOT_NOT_FOUND"));
    }
    let end = offset
        .saturating_add(SYNC_PAGE)
        .min(cached.snapshot.items.len());
    if end == cached.snapshot.items.len()
        && let Ok(mut cache) = snapshot_cache().lock()
        && let Some(value) = cache.snapshots.get_mut(&snapshot)
    {
        value.complete = true;
    }
    Ok(Json(
        json!({"revision":cached.revision,"snapshot_id":snapshot,"total":cached.snapshot.items.len(),"items":cached.snapshot.items[offset..end].to_vec(),"has_more":end<cached.snapshot.items.len(),"next_offset":if end<cached.snapshot.items.len(){Some(end)}else{None::<usize>}}),
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PosterRevision {
    expected_revision: u64,
}
async fn upload_poster(
    State(state): State<AppState>,
    Path((profile, media)): Path<(String, String)>,
    Query(query): Query<PosterRevision>,
    bytes: Bytes,
) -> Result<Json<Value>, ApiError> {
    let cache = state.config.data_dir.join("posters");
    let (media, revision) = state
        .database
        .call(move |s| {
            s.set_custom_poster(&profile, &media, query.expected_revision, &bytes, &cache)
        })
        .await?;
    Ok(Json(json!({"media":media,"revision":revision})))
}

async fn upload_series_poster(
    State(state): State<AppState>,
    Path((profile, media)): Path<(String, String)>,
    Query(query): Query<PosterRevision>,
    bytes: Bytes,
) -> Result<Json<Value>, ApiError> {
    let cache = state.config.data_dir.join("posters");
    let current = media.clone();
    let expected = query.expected_revision;
    let members = state
        .database
        .call(move |s| s.set_custom_series_poster(&profile, &media, expected, &bytes, &cache))
        .await?;
    let media = members
        .iter()
        .find(|item| item.id == current)
        .ok_or(ApiError(StatusCode::CONFLICT, "TMDB_SCOPE_INVALID"))?;
    Ok(Json(
        json!({"media":media,"members":members,"revision":expected + 1}),
    ))
}

async fn upload_season_poster(
    State(state): State<AppState>,
    Path((profile, media)): Path<(String, String)>,
    Query(query): Query<PosterRevision>,
    bytes: Bytes,
) -> Result<Json<Value>, ApiError> {
    let cache = state.config.data_dir.join("posters");
    let response_profile = profile.clone();
    let expected = query.expected_revision;
    let season = state
        .database
        .call(move |s| s.set_custom_season_poster(&profile, &media, expected, &bytes, &cache))
        .await?;
    let mut season = serde_json::to_value(season)
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "TMDB_SCOPE_INVALID"))?;
    let tv = season["tv_id"].as_i64().unwrap_or_default();
    let number = season["season"].as_i64().unwrap_or_default();
    if season["poster"].is_string() {
        season["poster"] = json!(format!(
            "/api/v1/profiles/{response_profile}/series/{tv}/seasons/{number}/artwork"
        ));
    }
    Ok(Json(json!({"season":season,"revision":expected + 1})))
}

/// Resolves only a catalogue-owned cached artwork path.  The reference exposed
/// by DTOs is this endpoint, never a local path or third-party URL.
async fn artwork(
    State(state): State<AppState>,
    Path((profile, media)): Path<(String, String)>,
) -> Result<Response, ApiError> {
    let path = state
        .database
        .call(move |s| s.artwork_path(&profile, &media))
        .await?;
    let cache = state.config.data_dir.join("posters");
    tokio::task::spawn_blocking(move || cached_artwork(&cache, &path))
        .await
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "ARTWORK_UNAVAILABLE"))?
}
async fn season_artwork(
    State(state): State<AppState>,
    Path((profile, tv, season)): Path<(String, i64, i32)>,
) -> Result<Response, ApiError> {
    let path = state
        .database
        .call(move |s| s.season_artwork_path(&profile, tv, season))
        .await?;
    let cache = state.config.data_dir.join("posters");
    tokio::task::spawn_blocking(move || cached_artwork(&cache, &path))
        .await
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "ARTWORK_UNAVAILABLE"))?
}
fn cached_artwork(cache: &std::path::Path, path: &std::path::Path) -> Result<Response, ApiError> {
    let (mut file, version) = crate::media_stream::open_indexed(cache, path)
        .map_err(|_| ApiError(StatusCode::NOT_FOUND, "ARTWORK_NOT_FOUND"))?;
    if version.size > 8 * 1024 * 1024 {
        return Err(ApiError(StatusCode::NOT_FOUND, "ARTWORK_NOT_FOUND"));
    }
    let mut bytes = Vec::new();
    file.by_ref()
        .take((8 * 1024 * 1024 + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| ApiError(StatusCode::NOT_FOUND, "ARTWORK_NOT_FOUND"))?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err(ApiError(StatusCode::NOT_FOUND, "ARTWORK_NOT_FOUND"));
    }
    let kind = if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        "image/jpeg"
    } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        "image/png"
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        "image/webp"
    } else {
        return Err(ApiError(StatusCode::NOT_FOUND, "ARTWORK_NOT_FOUND"));
    };
    Ok((
        [
            (header::CONTENT_TYPE, kind),
            (header::CACHE_CONTROL, "private, no-cache"),
        ],
        bytes,
    )
        .into_response())
}

async fn detail(
    State(state): State<AppState>,
    Path((profile, media)): Path<(String, String)>,
) -> Result<Json<Value>, ApiError> {
    let (item, revision) = state
        .database
        .call(move |store| {
            let item = store.media(&profile, &media)?;
            let revision = store.media_revision(&media)?;
            Ok((item, revision))
        })
        .await?;
    Ok(Json(json!({"media":item,"revision":revision})))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RevisionPatch {
    expected_revision: u64,
    #[serde(flatten)]
    patch: MediaPatch,
}
async fn update_media(
    State(state): State<AppState>,
    Path((profile, media)): Path<(String, String)>,
    Json(body): Json<RevisionPatch>,
) -> Result<Json<Value>, ApiError> {
    let (updated, revision) = state
        .database
        .call(move |store| {
            store.update_media_at_revision(&profile, &media, body.expected_revision, &body.patch)
        })
        .await?;
    Ok(Json(json!({"media":updated,"revision":revision})))
}
async fn series(
    State(state): State<AppState>,
    Path((profile, series)): Path<(String, String)>,
) -> Result<Json<Value>, ApiError> {
    let key = series.clone();
    let (members, revision, seasons) = state
        .database
        .call(move |store| {
            let members = store.series(&profile, &key)?;
            let tv_id = members.first().and_then(|m| m.tmdb_id);
            let mut seasons = serde_json::Map::new();
            if let Some(tv_id) = tv_id {
                for number in members
                    .iter()
                    .filter_map(|m| m.season)
                    .collect::<std::collections::BTreeSet<_>>()
                {
                    if let Some(metadata) = store.season_provider(tv_id, number)? {
                        let mut metadata = serde_json::to_value(metadata)
                            .map_err(|_| "Métadonnées de saison invalides.")?;
                        if metadata["poster"].is_string() {
                            metadata["poster"] = json!(format!(
                                "/api/v1/profiles/{profile}/series/{tv_id}/seasons/{number}/artwork"
                            ));
                        }
                        seasons.insert(number.to_string(), metadata);
                    }
                }
            }
            Ok((members, store.common_revision()?, seasons))
        })
        .await?;
    let total = members.len();
    let watched = members.iter().filter(|m| m.watched).count();
    Ok(Json(
        json!({"name":series,"revision":revision,"metadata":members.first().and_then(|m|m.series_metadata.clone()),"tmdb_id":members.first().and_then(|m|m.tmdb_id),"season_metadata":seasons,"episodes":members,"counters":{"total":total,"watched":watched,"unwatched":total-watched}}),
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SharedSeriesEdit {
    expected_revision: u64,
    patch: SeriesPatch,
}
async fn edit_series(
    State(state): State<AppState>,
    Path((profile, series)): Path<(String, String)>,
    Json(body): Json<SharedSeriesEdit>,
) -> Result<Json<Value>, ApiError> {
    let result = state
        .database
        .call(move |s| {
            let anchor = s
                .series(&profile, &series)?
                .first()
                .ok_or("Série introuvable.")?
                .id
                .clone();
            let key = format!("media:{anchor}");
            s.update_shared_series(&profile, &series, body.expected_revision, &body.patch)?;
            Ok((s.series(&profile, &key)?, s.common_revision()?, key))
        })
        .await?;
    Ok(Json(
        json!({"episodes":result.0,"revision":result.1,"key":result.2}),
    ))
}
#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
struct UnavailableQuery {
    offset: usize,
    limit: usize,
}
impl Default for UnavailableQuery {
    fn default() -> Self {
        Self {
            offset: 0,
            limit: 48,
        }
    }
}
async fn unavailable(
    State(state): State<AppState>,
    Query(q): Query<UnavailableQuery>,
) -> Result<Json<chill_domain::administration::UnavailablePage>, ApiError> {
    Ok(Json(
        state
            .database
            .call(move |s| s.unavailable_page(q.offset, q.limit))
            .await?,
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ForgetUnavailable {
    expected_epoch: u64,
    media_ids: Vec<String>,
    confirmed: bool,
}
async fn forget_unavailable(
    State(state): State<AppState>,
    Json(body): Json<ForgetUnavailable>,
) -> Result<Json<Value>, ApiError> {
    if !body.confirmed {
        return Err(ApiError(
            StatusCode::UNPROCESSABLE_ENTITY,
            "CONFIRMATION_REQUIRED",
        ));
    }
    state
        .database
        .call(move |s| s.forget_unavailable(&body.media_ids, body.expected_epoch))
        .await?;
    Ok(Json(json!({"removed":true})))
}
async fn related(
    State(state): State<AppState>,
    Path((profile, id)): Path<(String, String)>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(
        json!({"items":state.database.call(move|s|s.related(&profile,&id)).await?}),
    ))
}
async fn saga_artwork(
    State(state): State<AppState>,
    Path((profile, id)): Path<(String, String)>,
) -> Result<Response, ApiError> {
    let path = state
        .database
        .call(move |s| s.saga_artwork_path(&profile, &id))
        .await?;
    let cache = state.config.data_dir.join("posters");
    tokio::task::spawn_blocking(move || cached_artwork(&cache, &path))
        .await
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "ARTWORK_UNAVAILABLE"))?
}
async fn common_saga_artwork(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let path = state
        .database
        .call(move |s| s.common_saga_artwork_path(&id))
        .await?;
    let cache = state.config.data_dir.join("posters");
    tokio::task::spawn_blocking(move || cached_artwork(&cache, &path))
        .await
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "ARTWORK_UNAVAILABLE"))?
}
async fn upload_saga_poster(
    State(state): State<AppState>,
    Path((profile, id)): Path<(String, String)>,
    Query(q): Query<PosterRevision>,
    bytes: Bytes,
) -> Result<Json<Value>, ApiError> {
    let cache = state.config.data_dir.join("posters");
    let saga = state
        .database
        .call(move |s| {
            let path = chill_catalog::poster::store(&bytes, &cache)?;
            if let Err(e) =
                s.set_saga_artwork_at_revision(&profile, &id, q.expected_revision, &path)
            {
                let _ = std::fs::remove_file(&path);
                return Err(e);
            }
            s.saga(&id)
        })
        .await?;
    Ok(Json(json!({"saga":saga})))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MemberPoster {
    expected_revision: u64,
    media_id: String,
}
async fn saga_member_poster(
    State(state): State<AppState>,
    Path((profile, id)): Path<(String, String)>,
    Json(body): Json<MemberPoster>,
) -> Result<Json<Value>, ApiError> {
    let cache = state.config.data_dir.join("posters");
    let saga = state
        .database
        .call(move |s| {
            let members = s.saga(&id)?;
            if !members.media_ids.contains(&body.media_id) {
                return Err("Ce film n'appartient pas à cette saga.".into());
            }
            let source = s.artwork_path(&profile, &body.media_id)?;
            let (file, version) = crate::media_stream::open_indexed(&cache, &source)
                .map_err(|_| "Affiche inaccessible.")?;
            if version.size > 8 * 1024 * 1024 {
                return Err("Affiche trop volumineuse.".into());
            }
            let mut bytes = Vec::new();
            file.take(8 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| "Affiche inaccessible.")?;
            let path = chill_catalog::poster::store(&bytes, &cache)?;
            if let Err(e) =
                s.set_saga_artwork_at_revision(&profile, &id, body.expected_revision, &path)
            {
                let _ = std::fs::remove_file(&path);
                return Err(e);
            }
            s.saga(&id)
        })
        .await?;
    Ok(Json(json!({"saga":saga})))
}
async fn sagas(
    State(state): State<AppState>,
    Path(profile): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let items = state
        .database
        .call(move |store| {
            store.preferences(&profile)?;
            store.sagas()
        })
        .await?;
    Ok(Json(json!({"items":items})))
}
async fn saga(
    State(state): State<AppState>,
    Path((profile, id)): Path<(String, String)>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(
        json!({"saga":state.database.call(move|s|{s.preferences(&profile)?;s.saga(&id)}).await?}),
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SagaCreate {
    name: String,
}
async fn create_saga(
    State(state): State<AppState>,
    Path(profile): Path<String>,
    Json(body): Json<SagaCreate>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(
        json!({"saga":state.database.call(move|s|{s.preferences(&profile)?;s.create_saga(&body.name)}).await?}),
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SagaRename {
    expected_revision: u64,
    name: String,
}
async fn rename_saga(
    State(state): State<AppState>,
    Path((profile, id)): Path<(String, String)>,
    Json(body): Json<SagaRename>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(
        json!({"saga":state.database.call(move|s|{s.preferences(&profile)?;s.rename_saga_at_revision(&id,&body.name,body.expected_revision)}).await?}),
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SagaMembers {
    expected_revision: u64,
    media_ids: Vec<String>,
}
async fn add_saga_members(
    State(state): State<AppState>,
    Path((profile, id)): Path<(String, String)>,
    Json(body): Json<SagaMembers>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(
        json!({"saga":state.database.call(move|s|{s.preferences(&profile)?;s.saga_members_at_revision(&id,&body.media_ids,body.expected_revision,true)}).await?}),
    ))
}
async fn remove_saga_members(
    State(state): State<AppState>,
    Path((profile, id)): Path<(String, String)>,
    Json(body): Json<SagaMembers>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(
        json!({"saga":state.database.call(move|s|{s.preferences(&profile)?;s.saga_members_at_revision(&id,&body.media_ids,body.expected_revision,false)}).await?}),
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SagaOrder {
    expected_revision: u64,
    expected_order: Vec<String>,
    media_ids: Vec<String>,
}
async fn order_saga(
    State(state): State<AppState>,
    Path((profile, id)): Path<(String, String)>,
    Json(body): Json<SagaOrder>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(
        json!({"saga":state.database.call(move|s|{s.preferences(&profile)?;s.reorder_saga_at_revision(&id,body.expected_revision,&body.expected_order,&body.media_ids)}).await?}),
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SagaDelete {
    expected_revision: u64,
}
async fn delete_saga(
    State(state): State<AppState>,
    Path((profile, id)): Path<(String, String)>,
    Json(body): Json<SagaDelete>,
) -> Result<Json<Value>, ApiError> {
    state
        .database
        .call(move |s| {
            s.preferences(&profile)?;
            s.delete_saga_at_revision(&id, body.expected_revision)
        })
        .await?;
    Ok(Json(json!({"deleted":true})))
}
async fn playlists(
    State(state): State<AppState>,
    Path(profile): Path<String>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(
        json!({"items":state.database.call(move|s|s.playlists(&profile)).await?}),
    ))
}
async fn playlist(
    State(state): State<AppState>,
    Path((profile, playlist)): Path<(String, String)>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(
        json!({"playlist":state.database.call(move|s|s.playlist(&profile,&playlist)).await?}),
    ))
}
async fn playlist_artwork(
    State(state): State<AppState>,
    Path((profile, playlist)): Path<(String, String)>,
) -> Result<Response, ApiError> {
    let path = state
        .database
        .call(move |s| s.playlist_artwork_path(&profile, &playlist))
        .await?;
    let cache = state.config.data_dir.join("posters");
    tokio::task::spawn_blocking(move || cached_artwork(&cache, &path))
        .await
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "ARTWORK_UNAVAILABLE"))?
}
async fn upload_playlist_poster(
    State(state): State<AppState>,
    Path((profile, playlist)): Path<(String, String)>,
    Query(query): Query<PosterRevision>,
    bytes: Bytes,
) -> Result<Json<Value>, ApiError> {
    let cache = state.config.data_dir.join("posters");
    let view = state
        .database
        .call(move |s| {
            s.set_playlist_artwork_at_revision(
                &profile,
                &playlist,
                query.expected_revision,
                &bytes,
                &cache,
            )?;
            s.playlist(&profile, &playlist)
        })
        .await?;
    Ok(Json(json!({"playlist":view})))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RenamePlaylist {
    expected_revision: u64,
    name: String,
}
async fn rename_playlist(
    State(state): State<AppState>,
    Path((profile, playlist)): Path<(String, String)>,
    Json(body): Json<RenamePlaylist>,
) -> Result<Json<Value>, ApiError> {
    let view = state
        .database
        .call(move |s| {
            s.rename_playlist_at_revision(&profile, &playlist, &body.name, body.expected_revision)?;
            s.playlist(&profile, &playlist)
        })
        .await?;
    Ok(Json(json!({"playlist":view})))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlaylistCreate {
    name: String,
    media_id: Option<String>,
}
async fn create_playlist(
    State(state): State<AppState>,
    Path(profile): Path<String>,
    Json(body): Json<PlaylistCreate>,
) -> Result<Json<Value>, ApiError> {
    let view = state
        .database
        .call(move |s| {
            s.create_playlist_with_members(
                &profile,
                &body.name,
                &body.media_id.into_iter().collect::<Vec<_>>(),
            )
        })
        .await?;
    Ok(Json(json!({"playlist":view})))
}
async fn delete_playlist(
    State(state): State<AppState>,
    Path((profile, playlist)): Path<(String, String)>,
) -> Result<Json<Value>, ApiError> {
    let cache = state.config.data_dir.join("posters");
    state
        .database
        .call(move |s| s.delete_playlist_with_artwork(&profile, &playlist, &cache))
        .await?;
    Ok(Json(json!({"deleted":true})))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Members {
    expected_revision: u64,
    media_ids: Vec<String>,
}
async fn add_members(
    State(state): State<AppState>,
    Path((profile, playlist)): Path<(String, String)>,
    Json(body): Json<Members>,
) -> Result<Json<Value>, ApiError> {
    let view = state
        .database
        .call(move |s| {
            s.collection_member_add_bulk_at_revision(
                &profile,
                &playlist,
                &body.media_ids,
                body.expected_revision,
            )?;
            s.playlist(&profile, &playlist)
        })
        .await?;
    Ok(Json(json!({"playlist":view})))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Remove {
    expected_revision: u64,
}
async fn remove_member(
    State(state): State<AppState>,
    Path((profile, playlist, media)): Path<(String, String, String)>,
    Json(body): Json<Remove>,
) -> Result<Json<Value>, ApiError> {
    let view = state
        .database
        .call(move |s| {
            s.collection_member_remove_at_revision(
                &profile,
                &playlist,
                &media,
                Some(body.expected_revision),
            )?;
            s.playlist(&profile, &playlist)
        })
        .await?;
    Ok(Json(json!({"playlist":view})))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Order {
    expected_revision: u64,
    expected_order: Vec<String>,
    media_ids: Vec<String>,
}
async fn order_playlist(
    State(state): State<AppState>,
    Path((profile, playlist)): Path<(String, String)>,
    Json(body): Json<Order>,
) -> Result<Json<Value>, ApiError> {
    let view = state
        .database
        .call(move |s| {
            if s.playlist(&profile, &playlist)?.revision != body.expected_revision {
                return Err("La playlist a changé entre-temps.".into());
            };
            s.reorder_playlist(&profile, &playlist, &body.expected_order, &body.media_ids)?;
            s.playlist(&profile, &playlist)
        })
        .await?;
    Ok(Json(json!({"playlist":view})))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NextRequest {
    media_id: String,
    context: PlaybackContext,
}
async fn next_video(
    State(state): State<AppState>,
    Path(profile): Path<String>,
    Json(body): Json<NextRequest>,
) -> Result<Json<chill_domain::NextVideo>, ApiError> {
    Ok(Json(
        state
            .database
            .call(move |s| s.next_video(&profile, &body.media_id, &body.context))
            .await?,
    ))
}
async fn playback_info(
    State(state): State<AppState>,
    Path((profile, media)): Path<(String, String)>,
) -> Result<Json<chill_domain::PlaybackInfo>, ApiError> {
    Ok(Json(
        state
            .database
            .call(move |store| {
                let item = store.media(&profile, &media)?;
                let speed = match item.series.as_deref() {
                    Some(series) => store.series_speed(&profile, series)?,
                    None => 1.0,
                };
                Ok(chill_domain::PlaybackInfo {
                    media: item,
                    preferences: store.preferences(&profile)?,
                    speed,
                })
            })
            .await?,
    ))
}
async fn catalog(
    State(state): State<AppState>,
    Path(profile): Path<String>,
    Query(query): Query<BrowserQuery>,
) -> Result<Json<Value>, ApiError> {
    let page = state
        .database
        .call(move |store| store.browse_catalog(&profile, &query))
        .await?;
    Ok(Json(
        json!({"items":page.items,"has_more":page.has_more,"series_counters":page.series_counters}),
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Watched {
    watched: bool,
}
async fn watched(
    State(state): State<AppState>,
    Path((profile, media)): Path<(String, String)>,
    Json(body): Json<Watched>,
) -> Result<Json<Value>, ApiError> {
    let playback = state.playback.clone();
    state
        .database
        .call(move |store| {
            store.set_watched(&profile, &media, body.watched)?;
            if !body.watched {
                playback.reset_final(&profile, &media);
            }
            Ok(())
        })
        .await?;
    Ok(Json(json!({"saved":true})))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Rating {
    rating: Option<u8>,
    #[serde(default = "video_scope")]
    scope: String,
}
fn video_scope() -> String {
    "video".into()
}
async fn rating(
    State(state): State<AppState>,
    Path((profile, media)): Path<(String, String)>,
    Json(body): Json<Rating>,
) -> Result<Json<Value>, ApiError> {
    state
        .database
        .call(move |store| store.set_rating(&profile, &media, body.rating, &body.scope))
        .await?;
    Ok(Json(json!({"saved":true})))
}
async fn scan_status(State(state): State<AppState>) -> Result<Json<ScanStatus>, ApiError> {
    Ok(Json(
        state
            .scan
            .lock()
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "BUSY"))?
            .clone(),
    ))
}
async fn start_scan(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let root = {
        // Same lock order as library changes: library then scan; root cannot change mid-scan.
        let settings = state
            .library
            .lock()
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "BUSY"))?;
        let mut status = state
            .scan
            .lock()
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "BUSY"))?;
        if status.running {
            return Err(ApiError(StatusCode::CONFLICT, "SCAN_ACTIVE"));
        }
        let root = settings
            .video_root
            .clone()
            .ok_or(ApiError(StatusCode::BAD_REQUEST, "ROOT_REQUIRED"))?;
        *status = ScanStatus {
            running: true,
            ..ScanStatus::default()
        };
        root
    };
    tokio::spawn(async move {
        struct ScanGuard(AppState);
        impl Drop for ScanGuard {
            fn drop(&mut self) {
                if let Ok(mut s) = self.0.scan.lock() {
                    s.running = false;
                }
            }
        }
        let guard = ScanGuard(state.clone());
        let result = async {
            let exe = std::env::current_exe().map_err(|_| ())?;
            let child = tokio::process::Command::new(exe)
                .arg("--scan-worker")
                .arg(root)
                .stdin(Stdio::null())
                .stderr(Stdio::null())
                .stdout(Stdio::piped())
                .kill_on_drop(true)
                .spawn()
                .map_err(|_| ())?;
            let (exit, bytes) = chill_catalog::process::capture(
                child,
                chill_catalog::scanner::MAX_SCAN_BYTES,
                Duration::from_secs(300),
            )
            .await
            .map_err(|_| ())?;
            if !exit.success() {
                return Err(());
            }
            let media: Vec<chill_catalog::model::Media> =
                serde_json::from_slice(&bytes).map_err(|_| ())?;
            let count = media.len();
            // Catalogue merge/serialization does not occupy the interactive actor.
            // WAL readers remain usable; only the final atomic write owns SQLite's
            // write lock, and interactive writes have a shorter request deadline.
            let path = state.config.data_dir.join("catalog.sqlite");
            tokio::task::spawn_blocking(move || chill_domain::Store::open(&path)?.snapshot(media))
                .await
                .map_err(|_| ())?
                .map_err(|_| ())?;
            crate::fingerprint::enqueue_scan_candidates(state.clone()).await;
            crate::tmdb_api::enqueue_auto(state.clone()).await;
            Ok(count)
        }
        .await;
        if let Ok(mut status) = guard.0.scan.lock() {
            match result { Ok(count) => status.count = Some(count), Err(()) => status.last_error = Some("Scan impossible ou incomplet ; catalogue précédent conservé. Vérifiez le montage et les droits Linux.".into()) }
        }
    });
    Ok(Json(json!({"running":true})))
}
