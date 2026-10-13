use crate::api::{ApiError, AppState};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
};
use chill_domain::{PlaybackContext, store::CatalogQuery};
use chill_catalog::model::MediaPatch;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{process::Stdio, time::Duration};

#[derive(Clone, Default, Serialize)]
pub struct ScanStatus {
    pub running: bool,
    pub last_error: Option<String>,
    pub count: Option<usize>,
}
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/admin/scan", get(scan_status).post(start_scan))
        .route("/api/v1/profiles/{profile}/catalog", get(catalog))
        .route("/api/v1/profiles/{profile}/media/{media}", get(detail).put(update_media))
        .route("/api/v1/profiles/{profile}/media/{media}/subtitles", get(crate::media_stream::subtitles))
        .route("/api/v1/profiles/{profile}/media/{media}/subtitles/{id}", get(crate::media_stream::subtitle))
        .route("/api/v1/profiles/{profile}/series/{series}", get(series))
        .route("/api/v1/profiles/{profile}/sagas", get(sagas))
        .route("/api/v1/profiles/{profile}/playlists", get(playlists).post(create_playlist))
        .route("/api/v1/profiles/{profile}/playlists/{playlist}", get(playlist).put(rename_playlist).delete(delete_playlist))
        .route("/api/v1/profiles/{profile}/playlists/{playlist}/members", post(add_members))
        .route("/api/v1/profiles/{profile}/playlists/{playlist}/members/{media}", delete(remove_member))
        .route("/api/v1/profiles/{profile}/playlists/{playlist}/order", put(order_playlist))
        .route("/api/v1/profiles/{profile}/playback/next", post(next_video))
        .route(
            "/api/v1/profiles/{profile}/media/{media}/playback-info",
            get(playback_info),
        )
        .route(
            "/api/v1/profiles/{profile}/media/{media}/watched",
            put(watched),
        )
        .route(
            "/api/v1/profiles/{profile}/media/{media}/rating",
            put(rating),
        )
}

async fn detail(State(state): State<AppState>, Path((profile, media)): Path<(String, String)>) -> Result<Json<Value>, ApiError> {
    let (item,revision) = state.database.call(move |store| { let item=store.media(&profile, &media)?; let revision=store.media_revision(&media)?; Ok((item,revision)) }).await?;
    Ok(Json(json!({"media":item,"revision":revision})))
}

#[derive(Deserialize)] #[serde(deny_unknown_fields)]
struct RevisionPatch { expected_revision: u64, #[serde(flatten)] patch: MediaPatch }
async fn update_media(State(state): State<AppState>, Path((profile, media)): Path<(String,String)>, Json(body): Json<RevisionPatch>) -> Result<Json<Value>, ApiError> {
    let (updated,revision) = state.database.call(move |store| store.update_media_at_revision(&profile,&media,body.expected_revision,&body.patch)).await?;
    Ok(Json(json!({"media":updated,"revision":revision})))
}
async fn series(State(state): State<AppState>, Path((profile, series)): Path<(String,String)>) -> Result<Json<Value>, ApiError> {
    let key = series.clone();
    let members = state.database.call(move |store| store.series(&profile,&key)).await?;
    let total=members.len(); let watched=members.iter().filter(|m|m.watched).count();
    Ok(Json(json!({"name":series,"episodes":members,"counters":{"total":total,"watched":watched,"unwatched":total-watched}})))
}
async fn sagas(State(state): State<AppState>, Path(profile): Path<String>) -> Result<Json<Value>, ApiError> {
    let collections=state.database.call(move |store| store.collections(&profile)).await?;
    Ok(Json(json!({"items":collections.into_iter().filter(|c|c.kind=="saga").collect::<Vec<_>>()})))
}
async fn playlists(State(state): State<AppState>, Path(profile): Path<String>) -> Result<Json<Value>, ApiError> { Ok(Json(json!({"items":state.database.call(move|s|s.playlists(&profile)).await?}))) }
async fn playlist(State(state): State<AppState>, Path((profile,playlist)): Path<(String,String)>) -> Result<Json<Value>, ApiError> { Ok(Json(json!({"playlist":state.database.call(move|s|s.playlist(&profile,&playlist)).await?}))) }
#[derive(Deserialize)] #[serde(deny_unknown_fields)] struct RenamePlaylist { expected_revision:u64, name:String }
async fn rename_playlist(State(state):State<AppState>,Path((profile,playlist)):Path<(String,String)>,Json(body):Json<RenamePlaylist>)->Result<Json<Value>,ApiError>{let view=state.database.call(move|s|{s.rename_playlist_at_revision(&profile,&playlist,&body.name,body.expected_revision)?;s.playlist(&profile,&playlist)}).await?;Ok(Json(json!({"playlist":view})))}
#[derive(Deserialize)] #[serde(deny_unknown_fields)] struct PlaylistCreate { name:String, media_id:Option<String> }
async fn create_playlist(State(state): State<AppState>, Path(profile): Path<String>, Json(body): Json<PlaylistCreate>) -> Result<Json<Value>,ApiError> { let view=state.database.call(move|s| s.create_playlist_with_members(&profile,&body.name,&body.media_id.into_iter().collect::<Vec<_>>())).await?; Ok(Json(json!({"playlist":view}))) }
async fn delete_playlist(State(state): State<AppState>, Path((profile,playlist)): Path<(String,String)>) -> Result<Json<Value>,ApiError> { state.database.call(move|s|s.delete_collection(&profile,&playlist)).await?; Ok(Json(json!({"deleted":true}))) }
#[derive(Deserialize)] #[serde(deny_unknown_fields)] struct Members { expected_revision:u64, media_ids:Vec<String> }
async fn add_members(State(state): State<AppState>, Path((profile,playlist)):Path<(String,String)>, Json(body):Json<Members>) -> Result<Json<Value>,ApiError> { let view=state.database.call(move|s| { s.collection_member_add_bulk_at_revision(&profile,&playlist,&body.media_ids,body.expected_revision)?; s.playlist(&profile,&playlist) }).await?; Ok(Json(json!({"playlist":view}))) }
#[derive(Deserialize)] #[serde(deny_unknown_fields)] struct Remove { expected_revision:u64 }
async fn remove_member(State(state):State<AppState>,Path((profile,playlist,media)):Path<(String,String,String)>,Json(body):Json<Remove>)->Result<Json<Value>,ApiError>{let view=state.database.call(move|s|{s.collection_member_remove_at_revision(&profile,&playlist,&media,Some(body.expected_revision))?;s.playlist(&profile,&playlist)}).await?;Ok(Json(json!({"playlist":view})))}
#[derive(Deserialize)] #[serde(deny_unknown_fields)] struct Order { expected_revision:u64, expected_order:Vec<String>, media_ids:Vec<String> }
async fn order_playlist(State(state):State<AppState>,Path((profile,playlist)):Path<(String,String)>,Json(body):Json<Order>)->Result<Json<Value>,ApiError>{let view=state.database.call(move|s|{if s.playlist(&profile,&playlist)?.revision!=body.expected_revision{return Err("La playlist a changé entre-temps.".into())};s.reorder_playlist(&profile,&playlist,&body.expected_order,&body.media_ids)?;s.playlist(&profile,&playlist)}).await?;Ok(Json(json!({"playlist":view})))}
#[derive(Deserialize)] #[serde(deny_unknown_fields)] struct NextRequest { media_id:String, context:PlaybackContext }
async fn next_video(State(state):State<AppState>,Path(profile):Path<String>,Json(body):Json<NextRequest>)->Result<Json<chill_domain::NextVideo>,ApiError>{Ok(Json(state.database.call(move|s|s.next_video(&profile,&body.media_id,&body.context)).await?))}

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
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct PageQuery {
    offset: usize,
    limit: usize,
    category: Option<String>,
    q: Option<String>,
}
async fn catalog(
    State(state): State<AppState>,
    Path(profile): Path<String>,
    Query(query): Query<PageQuery>,
) -> Result<Json<Value>, ApiError> {
    let page = state
        .database
        .call(move |store| {
            store.catalog_page(
                &profile,
                &CatalogQuery {
                    offset: query.offset,
                    limit: query.limit,
                    category: query.category,
                    title_contains: query.q,
                },
            )
        })
        .await?;
    Ok(Json(json!({"items":page.items,"has_more":page.has_more})))
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
fn video_scope() -> String { "video".into() }
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
            Ok(count)
        }
        .await;
        if let Ok(mut status) = guard.0.scan.lock() {
            match result { Ok(count) => status.count = Some(count), Err(()) => status.last_error = Some("Scan impossible ou incomplet ; catalogue précédent conservé. Vérifiez le montage et les droits Linux.".into()) }
        }
    });
    Ok(Json(json!({"running":true})))
}
