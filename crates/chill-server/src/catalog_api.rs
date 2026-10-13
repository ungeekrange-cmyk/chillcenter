use crate::api::{ApiError, AppState};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, put},
};
use chill_domain::store::CatalogQuery;
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
        .route(
            "/api/v1/profiles/{profile}/media/{media}/watched",
            put(watched),
        )
        .route(
            "/api/v1/profiles/{profile}/media/{media}/rating",
            put(rating),
        )
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
    state
        .database
        .call(move |store| store.set_watched(&profile, &media, body.watched))
        .await?;
    Ok(Json(json!({"saved":true})))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Rating {
    rating: Option<u8>,
}
async fn rating(
    State(state): State<AppState>,
    Path((profile, media)): Path<(String, String)>,
    Json(body): Json<Rating>,
) -> Result<Json<Value>, ApiError> {
    state
        .database
        .call(move |store| store.set_rating(&profile, &media, body.rating, "video"))
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
