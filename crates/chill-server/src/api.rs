//! Bounded SQLite actor: no database call blocks either of the two Tokio workers.
use crate::{
    config::{LibrarySettings, ServerConfig},
    tls::CertificateSchedule,
};
use axum::{
    Json, Router,
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, put},
};
use chill_domain::{Preferences, Profile, Store};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::{mpsc, oneshot};

type Job = Box<dyn FnOnce(&mut Store) + Send>;
#[derive(Clone)]
pub struct DatabaseActor {
    sender: mpsc::Sender<Job>,
    pub(crate) sync_generation: Arc<str>,
}
struct Cancellation(Arc<AtomicBool>);
impl Drop for Cancellation {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}
impl DatabaseActor {
    pub fn open(path: PathBuf) -> Result<Self, String> {
        let sync_generation = crate::sync_api::database_generation(&path)?;
        let (sender, mut receiver) = mpsc::channel::<Job>(16);
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("chill-sqlite".into())
            .spawn(move || {
                let mut store = match Store::open(&path).and_then(|store| {
                    // The service holds its exclusive data lock before opening
                    // this actor. No worker from the prior process survives.
                    store.recover_interrupted_fingerprints()?;
                    Ok(store)
                }) {
                    Ok(store) => {
                        let _ = ready_tx.send(Ok(()));
                        store
                    }
                    Err(_) => {
                        let _ = ready_tx.send(Err("Base de données inaccessible.".to_string()));
                        return;
                    }
                };
                while let Some(job) = receiver.blocking_recv() {
                    job(&mut store);
                }
            })
            .map_err(|_| "Initialisation de la base impossible.")?;
        ready_rx
            .recv()
            .map_err(|_| "Initialisation de la base interrompue.")??;
        Ok(Self {
            sender,
            sync_generation: sync_generation.into(),
        })
    }
    pub async fn call<T: Send + 'static>(
        &self,
        job: impl FnOnce(&mut Store) -> Result<T, String> + Send + 'static,
    ) -> Result<T, ApiError> {
        self.call_with_budget(Duration::from_secs(10), job).await
    }
    pub(crate) async fn call_with_budget<T: Send + 'static>(
        &self,
        budget: Duration,
        job: impl FnOnce(&mut Store) -> Result<T, String> + Send + 'static,
    ) -> Result<T, ApiError> {
        let (tx, rx) = oneshot::channel();
        // Shorter than both the native client's 20s and middleware's 30s budget.
        let deadline = Instant::now() + budget;
        let cancelled = Arc::new(AtomicBool::new(false));
        let guard = Cancellation(cancelled.clone());
        self.sender
            .try_send(Box::new(move |store| {
                if tx.is_closed() {
                    return;
                }
                if Instant::now() >= deadline {
                    let _ = tx.send(Err("REQUEST_TIMEOUT".into()));
                    return;
                }
                let probe: Arc<dyn Fn() -> bool + Send + Sync> = Arc::new(move || {
                    cancelled.load(Ordering::Acquire) || Instant::now() >= deadline
                });
                let result = store
                    .cancellation_probe(Some(probe.clone()))
                    .and_then(|()| job(store));
                let result = match store.cancellation_probe(None) {
                    Ok(()) => result,
                    Err(error) => Err(error),
                };
                let _ = tx.send(if probe() {
                    Err("REQUEST_TIMEOUT".into())
                } else {
                    result
                });
            }))
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "BUSY"))?;
        let result = tokio::time::timeout(budget, rx)
            .await
            .map_err(|_| ApiError(StatusCode::REQUEST_TIMEOUT, "REQUEST_TIMEOUT"))?
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "DATABASE_OFFLINE"))?
            .map_err(|error| ApiError::from(chill_domain::StoreFailure::from(error)));
        drop(guard);
        result
    }
}

#[derive(Clone)]
pub struct AppState {
    pub playback: crate::playback::PlaybackService,
    pub config: Arc<ServerConfig>,
    pub library: Arc<Mutex<LibrarySettings>>,
    pub database: DatabaseActor,
    pub certificate: Arc<Mutex<CertificateSchedule>>,
    pub scan: Arc<Mutex<crate::catalog_api::ScanStatus>>,
    pub tmdb_jobs: Arc<crate::tmdb_api::TmdbJobService>,
}

#[derive(Debug)]
pub struct ApiError(pub StatusCode, pub &'static str);
impl From<chill_domain::StoreFailure> for ApiError {
    fn from(error: chill_domain::StoreFailure) -> Self {
        use chill_domain::StoreFailure::*;
        match error {
            SyncChanged => Self(StatusCode::CONFLICT, "SYNC_SNAPSHOT_CHANGED"),
            SyncTooLarge => Self(StatusCode::PAYLOAD_TOO_LARGE, "SYNC_TOO_LARGE"),
            ArtworkNotFound => Self(StatusCode::NOT_FOUND, "ARTWORK_NOT_FOUND"),
            ProfileNotFound => Self(StatusCode::NOT_FOUND, "PROFILE_NOT_FOUND"),
            MediaNotFound => Self(StatusCode::NOT_FOUND, "MEDIA_NOT_FOUND"),
            ProfileConflict => Self(StatusCode::CONFLICT, "PROFILE_CONFLICT"),
            ProfileNameTaken => Self(StatusCode::CONFLICT, "PROFILE_NAME_TAKEN"),
            ImportPlanStale => Self(StatusCode::CONFLICT, "IMPORT_PLAN_STALE"),
            DatabaseOffline => Self(StatusCode::SERVICE_UNAVAILABLE, "DATABASE_OFFLINE"),
            RequestTimeout => Self(StatusCode::REQUEST_TIMEOUT, "REQUEST_TIMEOUT"),
            FingerprintBusy => Self(StatusCode::SERVICE_UNAVAILABLE, "FINGERPRINT_BUSY"),
            AiConfirmationRequired => {
                Self(StatusCode::UNPROCESSABLE_ENTITY, "AI_CONFIRMATION_REQUIRED")
            }
            AiPlanInvalid => Self(StatusCode::UNPROCESSABLE_ENTITY, "AI_PLAN_INVALID"),
            InvalidOperation => Self(StatusCode::UNPROCESSABLE_ENTITY, "INVALID_OPERATION"),
        }
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({"code":self.1,"message":"Opération refusée ou temporairement indisponible.","request_id":uuid::Uuid::new_v4().to_string()}))).into_response()
    }
}

fn api_routes() -> Router<AppState> {
    Router::new()
        .merge(crate::ai_api::routes())
        .merge(crate::archive_api::routes())
        .merge(crate::export_jobs::routes())
        .merge(crate::catalog_api::routes())
        .merge(crate::sync_api::routes())
        .merge(crate::identity_api::routes())
        .merge(crate::tmdb_api::routes())
        .merge(crate::playback::routes())
        .merge(crate::updates_api::routes())
        .route("/server", get(server))
        .route("/admin/library", get(library).put(update_library))
        .route("/profiles", get(profiles).post(create_profile))
        .route(
            "/profiles/{profile}",
            put(rename_profile).delete(delete_profile),
        )
        .route(
            "/profiles/{profile}/preferences",
            get(preferences).put(save_preferences),
        )
        .route("/openapi.json", get(crate::api_version::openapi))
}

/// Routes that carry secrets or whole archives never go over the clear-text listener (ADR-0018); the web client does not use them.
fn https_only_path(path: &str) -> bool {
    let rest = path
        .strip_prefix("/api/")
        .and_then(|p| p.split_once('/'))
        .map_or("", |(_, rest)| rest)
        .trim_start_matches('/');
    ["admin/tmdb", "admin/exports", "admin/imports"]
        .iter()
        .any(|prefix| {
            rest == *prefix
                || rest
                    .strip_prefix(prefix)
                    .is_some_and(|r| r.starts_with('/'))
        })
}

/// Which listener a request came in on. The plain-HTTP listener (ADR-0018) serves the bundled web client only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scheme {
    Https,
    Http,
}

pub fn router(state: AppState) -> Router {
    build_router(state, Scheme::Https)
}

pub fn router_http(state: AppState) -> Router {
    build_router(state, Scheme::Http)
}

fn build_router(state: AppState, scheme: Scheme) -> Router {
    let api = api_routes();
    Router::new()
        .nest(crate::api_version::PREFIX, api)
        .fallback(crate::web_static::serve)
        .layer(axum::extract::DefaultBodyLimit::max(1024 * 1024))
        .layer(middleware::from_fn_with_state(state.clone(), same_origin))
        .layer(axum::Extension(scheme))
        .layer(middleware::from_fn(crate::api_version::normalize_response))
        .with_state(state)
}

/// No CORS. Native Rust requests have no Origin. A browser request is accepted
/// only when it is same-origin with this server (the bundled web client, ADR-0014):
/// `Sec-Fetch-Site: same-origin`, and `Origin`, when present, equal to `https://<Host>`.
/// Everything else (cross-site, same-site, other origins) is refused. Host must name this server.
async fn same_origin(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    // Synchronous checks live in their own function: no borrow of the request may stay alive across
    // the `.await` below, otherwise the middleware future stops being `Send`.
    check_browser_origin_and_host(&state, &request)?;
    let _mutation = crate::updates_api::begin_mutation(request.method());
    if crate::updates_api::maintenance_generation().is_some()
        && !crate::updates_api::admitted_during_maintenance(request.method(), request.uri().path())
    {
        return Err(ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "UPDATE_MAINTENANCE",
        ));
    }
    // Current routes are JSON only. Future media routes need per-read deadlines,
    // not a total playback deadline.
    tokio::time::timeout(std::time::Duration::from_secs(30), next.run(request))
        .await
        .map_err(|_| ApiError(StatusCode::REQUEST_TIMEOUT, "REQUEST_TIMEOUT"))
}

fn check_browser_origin_and_host(
    state: &AppState,
    request: &Request<Body>,
) -> Result<(), ApiError> {
    let host = request
        .headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .or_else(|| request.uri().authority().map(|a| a.as_str()))
        .ok_or(ApiError(StatusCode::BAD_REQUEST, "HOST_REQUIRED"))?;
    let scheme = request
        .extensions()
        .get::<Scheme>()
        .copied()
        .unwrap_or(Scheme::Https);
    let (prefix, default_port, listen_port) = match scheme {
        Scheme::Https => ("https", 443, state.config.listen.port()),
        Scheme::Http => ("http", 80, state.config.http_listen.map_or(0, |a| a.port())),
    };
    if scheme == Scheme::Http && https_only_path(request.uri().path()) {
        return Err(ApiError(StatusCode::FORBIDDEN, "HTTPS_ONLY"));
    }
    let expected_origin = format!("{prefix}://{host}");
    let header = |name: &str| request.headers().get(name).map(|v| v.to_str().ok());
    let browser_allowed = match (header("sec-fetch-site"), header("origin")) {
        (None, None) => true,
        (Some(Some("same-origin")), None) => true,
        (Some(Some("same-origin")), Some(Some(origin))) => {
            origin.eq_ignore_ascii_case(&expected_origin)
        }
        // Browsers send no `Sec-Fetch-*` to a plain-HTTP origin that is not "potentially trustworthy" (a LAN address is
        // not), but they always send `Origin` on non-GET requests: it must be this server's own origin (ADR-0018).
        (None, Some(Some(origin))) if scheme == Scheme::Http => {
            origin.eq_ignore_ascii_case(&expected_origin)
        }
        // `none` is a user-typed navigation: static page only, never the API.
        (Some(Some("none")), None) => {
            !request.uri().path().starts_with("/api/")
                && request.method() == axum::http::Method::GET
        }
        _ => false,
    };
    if !browser_allowed {
        return Err(ApiError(StatusCode::FORBIDDEN, "BROWSER_ORIGIN_REFUSED"));
    }
    let authority: axum::http::uri::Authority = host
        .parse()
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "HOST_INVALID"))?;
    let raw_host = authority
        .host()
        .trim_start_matches('[')
        .trim_end_matches(']');
    // The default HTTPS port is also accepted: a network-level redirect 443 -> listen port (ADR-0017) makes
    // browsers send `Host: <name>` without a port. The name is still checked against the certificate names.
    let port = authority.port_u16().unwrap_or(default_port);
    if !state
        .config
        .tls_names
        .iter()
        .any(|n| n.eq_ignore_ascii_case(raw_host))
        || (port != listen_port && port != default_port)
    {
        return Err(ApiError(StatusCode::FORBIDDEN, "HOST_REFUSED"));
    }
    Ok(())
}

async fn server(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let certificate = state
        .certificate
        .lock()
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "BUSY"))?
        .clone();
    Ok(Json(
        json!({"server_id":certificate.server_id,"api_version":crate::api_version::VERSION,"supported_api_versions":[crate::api_version::VERSION],"version":env!("CARGO_PKG_VERSION"),"capabilities":["profiles","configurable_library","multi_library_locations","catalog_pages","manual_personal_state","scan","playback_leases","media_range","progress","tmdb_configuration","tmdb_jobs","ai_filters","openapi"],"tls":{"renew_at":certificate.renew_at,"expires_at":certificate.expires_at}}),
    ))
}

async fn library(State(state): State<AppState>) -> Result<Json<LibrarySettings>, ApiError> {
    Ok(Json(
        state
            .library
            .lock()
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "BUSY"))?
            .clone(),
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LibraryUpdate {
    video_root: Option<PathBuf>,
    /// Omitted by legacy clients: retain already configured extra locations.
    #[serde(default)]
    additional_video_roots: Option<Vec<PathBuf>>,
    expected_revision: u64,
}
async fn update_library(
    State(state): State<AppState>,
    Json(body): Json<LibraryUpdate>,
) -> Result<Json<LibrarySettings>, ApiError> {
    tokio::task::spawn_blocking(move || {
        let mut settings = state
            .library
            .lock()
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "BUSY"))?;
        if state
            .scan
            .lock()
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "BUSY"))?
            .running
        {
            return Err(ApiError(StatusCode::CONFLICT, "SCAN_ACTIVE"));
        }
        let before = settings.revision;
        let result = settings.update(
            &state.config,
            body.video_root,
            body.additional_video_roots,
            body.expected_revision,
        );
        // A rename followed by failed directory fsync still published a new root.
        // Invalidate old grants even when durable persistence cannot be promised.
        if settings.revision != before {
            state.playback.clear();
        }
        result.map_err(|error| {
            if error == "CONFIG_CONFLICT" {
                ApiError(StatusCode::CONFLICT, "CONFIG_CONFLICT")
            } else if error == "CONFIG_DURABILITY_UNKNOWN" {
                ApiError(StatusCode::SERVICE_UNAVAILABLE, "CONFIG_DURABILITY_UNKNOWN")
            } else {
                ApiError(StatusCode::BAD_REQUEST, "CONFIG_INVALID_OR_UNWRITABLE")
            }
        })?;
        state.playback.clear();
        Ok(Json(settings.clone()))
    })
    .await
    .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "BUSY"))?
}

async fn profiles(State(state): State<AppState>) -> Result<Json<Vec<Profile>>, ApiError> {
    Ok(Json(state.database.call(|store| store.profiles()).await?))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Name {
    name: String,
    #[serde(default)]
    initial_preferences: Option<Preferences>,
}
async fn create_profile(
    State(state): State<AppState>,
    Json(body): Json<Name>,
) -> Result<(StatusCode, Json<Profile>), ApiError> {
    Ok((
        StatusCode::CREATED,
        Json(
            state
                .database
                .call(move |store| {
                    store.create_profile_with_preferences(&body.name, body.initial_preferences)
                })
                .await?,
        ),
    ))
}
async fn rename_profile(
    State(state): State<AppState>,
    Path(profile): Path<String>,
    Json(body): Json<Rename>,
) -> Result<Json<Profile>, ApiError> {
    Ok(Json(
        state
            .database
            .call(move |store| {
                store.rename_profile_at_revision(&profile, &body.name, body.expected_revision)
            })
            .await?,
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Rename {
    name: String,
    expected_revision: u64,
}
async fn delete_profile(
    State(state): State<AppState>,
    Path(profile): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    if headers
        .get("x-chill-confirm-delete")
        .and_then(|h| h.to_str().ok())
        != Some(profile.as_str())
    {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "DELETE_CONFIRMATION_REQUIRED",
        ));
    }
    let playback = state.playback.clone();
    state
        .database
        .call(move |store| {
            store.delete_profile(&profile)?;
            playback.revoke_profile(&profile);
            Ok(())
        })
        .await?;
    Ok(Json(json!({"deleted":true})))
}
async fn preferences(
    State(state): State<AppState>,
    Path(profile): Path<String>,
) -> Result<Json<Preferences>, ApiError> {
    Ok(Json(
        state
            .database
            .call(move |store| store.preferences(&profile))
            .await?,
    ))
}
async fn save_preferences(
    State(state): State<AppState>,
    Path(profile): Path<String>,
    Json(body): Json<Preferences>,
) -> Result<Json<Preferences>, ApiError> {
    let result = state
        .database
        .call(move |store| {
            store.save_preferences(&profile, &body)?;
            Ok(body)
        })
        .await?;
    Ok(Json(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    async fn json_call(
        app: &Router,
        method: &str,
        path: &str,
        value: Value,
    ) -> (StatusCode, Value) {
        let response = app
            .clone()
            .oneshot(request(method, path, value))
            .await
            .unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }
    #[tokio::test]
    async fn catalog_mirror_is_coherent_scoped_and_ignores_unrelated_progress() {
        let (dir, app) = setup();
        let (_, a) = json_call(
            &app,
            "POST",
            "/api/2026-10-14/profiles",
            json!({"name":"A","initial_preferences":{"autoplay_seconds":20}}),
        )
        .await;
        let (_, b) = json_call(
            &app,
            "POST",
            "/api/2026-10-14/profiles",
            json!({"name":"B"}),
        )
        .await;
        let a = a["id"].as_str().unwrap();
        let b = b["id"].as_str().unwrap();
        assert_eq!(
            json_call(
                &app,
                "GET",
                &format!("/api/2026-10-14/profiles/{a}/preferences"),
                Value::Null
            )
            .await
            .1["autoplay_seconds"],
            20
        );
        assert_eq!(
            json_call(
                &app,
                "GET",
                &format!("/api/2026-10-14/profiles/{b}/preferences"),
                Value::Null
            )
            .await
            .1["autoplay_seconds"],
            10
        );
        let mut store = Store::open(&dir.path().join("catalog.sqlite")).unwrap();
        let media=(0..201).map(|i|serde_json::from_value(json!({"id":format!("film-{i:03}.mkv"),"path":format!("/fixture/film-{i:03}.mkv"),"title":format!("Film {i}"),"category":"film","year":2026,"genres":[],"overview":"fixture","poster":null,"saga":null,"watched":false,"available":true,"added_at":1,"tmdb_id":null})).unwrap()).collect();
        store.snapshot(media).unwrap();
        let endpoint = format!("/api/2026-10-14/profiles/{a}/catalog/snapshot");
        let (status, first) = json_call(&app, "POST", &endpoint, json!({})).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(first["items"].as_array().unwrap().len(), 100);
        assert_eq!(first["total"], 201);
        let rev = first["revision"].as_str().unwrap();
        let id = first["items"][0]["media"]["id"].as_str().unwrap();
        assert!(!first.to_string().contains("/fixture/"));
        let sql = rusqlite::Connection::open(dir.path().join("catalog.sqlite")).unwrap();
        sql.execute("INSERT INTO cc_profile_state(profile_id,stable_media_id,position_ms) VALUES(?1,?2,2000)",rusqlite::params![a,id]).unwrap();
        store.set_watched(b, id, true).unwrap();
        store
            .save_preferences(
                a,
                &Preferences {
                    autoplay_seconds: Some(15),
                    ..Preferences::default()
                },
            )
            .unwrap();
        let check = json_call(&app, "POST", &endpoint, json!({"known_revision":rev}))
            .await
            .1;
        assert_eq!(
            check["unchanged"], true,
            "progress/preferences/other profile must not trigger catalogue download"
        );
        let before = store.sync_revision(a).unwrap();
        store.set_watched(a, id, true).unwrap();
        assert_ne!(store.sync_revision(a).unwrap(), before);
        let changed = json_call(&app, "POST", &endpoint, json!({"known_revision":rev}))
            .await
            .1;
        assert!(changed["items"].is_array());
        assert_ne!(changed["revision"], rev);
        // Three cold sessions must coexist; never evict an active transfer.
        let third = json_call(
            &app,
            "POST",
            &format!("/api/2026-10-14/profiles/{b}/catalog/snapshot"),
            json!({"known_revision":rev}),
        )
        .await;
        assert_eq!(third.0, StatusCode::OK);
        assert!(third.1["items"].is_array());
        assert_eq!(
            json_call(&app, "POST", &endpoint, json!({})).await.0,
            StatusCode::TOO_MANY_REQUESTS
        );
        let snapshot = first["snapshot_id"].as_str().unwrap();
        let page = json_call(
            &app,
            "GET",
            &format!("{endpoint}/{snapshot}?offset=100"),
            Value::Null,
        )
        .await
        .1;
        assert_eq!(page["revision"], rev);
        assert_eq!(page["items"].as_array().unwrap().len(), 100);
        assert_eq!(
            json_call(
                &app,
                "GET",
                &format!("/api/2026-10-14/profiles/{b}/catalog/snapshot/{snapshot}?offset=100"),
                Value::Null
            )
            .await
            .0,
            StatusCode::NOT_FOUND
        );
        let final_page = json_call(
            &app,
            "GET",
            &format!("{endpoint}/{snapshot}?offset=200"),
            Value::Null,
        )
        .await
        .1;
        assert_eq!(final_page["items"].as_array().unwrap().len(), 1);
        assert_eq!(final_page["has_more"], false);
        store.delete_profile(a).unwrap();
        assert_eq!(
            json_call(
                &app,
                "GET",
                &format!("{endpoint}/{snapshot}?offset=100"),
                Value::Null
            )
            .await
            .0,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            json_call(&app, "POST", &endpoint, json!({"known_revision":rev}))
                .await
                .0,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            json_call(
                &app,
                "GET",
                &format!(
                    "/api/2026-10-14/profiles/{b}/catalog/snapshot/{snapshot}?offset={}",
                    usize::MAX
                ),
                Value::Null
            )
            .await
            .0,
            StatusCode::NOT_FOUND
        );
    }
    fn setup() -> (tempfile::TempDir, Router) {
        setup_with(false)
    }
    fn setup_with(http: bool) -> (tempfile::TempDir, Router) {
        let dir = tempfile::tempdir().unwrap();
        let config = ServerConfig {
            data_dir: dir.path().to_owned(),
            http_listen: http.then(|| "127.0.0.1:8080".parse().unwrap()),
            ..ServerConfig::default()
        };
        let certificates = crate::tls::ensure(&config, time::OffsetDateTime::now_utc()).unwrap();
        let state = AppState {
            playback: Default::default(),
            tmdb_jobs: Arc::new(crate::tmdb_api::TmdbJobService::default()),
            scan: Arc::new(Mutex::new(crate::catalog_api::ScanStatus::default())),
            database: DatabaseActor::open(dir.path().join("catalog.sqlite")).unwrap(),
            library: Arc::new(Mutex::new(LibrarySettings::load(&config).unwrap())),
            config: Arc::new(config),
            certificate: Arc::new(Mutex::new(certificates.schedule)),
        };
        (
            dir,
            if http {
                router_http(state)
            } else {
                router(state)
            },
        )
    }
    #[tokio::test]
    async fn legacy_titles_equal_to_opaque_keys_cannot_select_or_edit_another_group() {
        for use_media_key in [false, true] {
            let (dir, app) = setup();
            let mut store = Store::open(&dir.path().join("catalog.sqlite")).unwrap();
            let profile = store.create_profile("Selector").unwrap();
            let items = [1001, 2002]
                .into_iter()
                .map(|tv| {
                    let mut m = chill_catalog::scanner::parse_file(
                        std::path::Path::new("/fixture"),
                        &std::path::PathBuf::from(format!("/fixture/Show{tv}/S01E01.mkv")),
                    );
                    m.category = "serie".into();
                    m.series = Some(format!("Show{tv}"));
                    m.tmdb_id = Some(tv);
                    m.season = Some(1);
                    m.episode = Some(1);
                    m
                })
                .collect();
            store.snapshot(items).unwrap();
            let items = store.catalog(&profile.id).unwrap();
            let target = items.iter().find(|m| m.tmdb_id == Some(1001)).unwrap();
            let literal = items.iter().find(|m| m.tmdb_id == Some(2002)).unwrap();
            let key = if use_media_key {
                format!("media:{}", target.id)
            } else {
                "tmdb:tv:1001".into()
            };
            store
                .update_media_at_revision(
                    &profile.id,
                    &literal.id,
                    store.media_revision(&literal.id).unwrap(),
                    &chill_catalog::model::MediaPatch {
                        series: Some(key.clone()),
                        ..Default::default()
                    },
                )
                .unwrap();
            for version in ["2026-10-14"] {
                let endpoint = format!("/api/{version}/profiles/{}/series/{key}", profile.id);
                for (suffix, tv) in [
                    ("", 2002),
                    ("?selector=title", 2002),
                    ("?selector=key", 1001),
                ] {
                    let (status, view) =
                        json_call(&app, "GET", &(endpoint.clone() + suffix), Value::Null).await;
                    assert_eq!(status, StatusCode::OK);
                    assert_eq!(view["episodes"].as_array().unwrap().len(), 1);
                    assert_eq!(view["episodes"][0]["tmdb_id"], tv);
                }
                for (suffix, tv, overview) in [
                    ("", 2002, "Legacy literal title"),
                    ("?selector=key", 1001, "Explicit opaque target"),
                ] {
                    let before = store.catalog(&profile.id).unwrap();
                    let patch_name = before
                        .iter()
                        .find(|m| m.tmdb_id == Some(tv))
                        .unwrap()
                        .series
                        .clone()
                        .unwrap();
                    let body = json!({"expected_revision":store.common_revision().unwrap(),"patch":{"name":patch_name,"year":0,"genres":[],"overview":overview,"cast":[],"directors":[]}});
                    let (status, _) =
                        json_call(&app, "PUT", &(endpoint.clone() + suffix), body).await;
                    assert_eq!(status, StatusCode::OK);
                    let after = store.catalog(&profile.id).unwrap();
                    for old in before {
                        let new = after.iter().find(|m| m.id == old.id).unwrap();
                        if old.tmdb_id == Some(tv) {
                            assert_eq!(new.series_overview, overview);
                        } else {
                            assert_eq!(
                                new, &old,
                                "Unselected group must remain completely unchanged"
                            );
                        }
                        assert_eq!(new.tmdb_id, old.tmdb_id);
                        assert_eq!(new.path, old.path);
                    }
                }
                assert_eq!(
                    json_call(&app, "GET", &(endpoint + "?selector=invalid"), Value::Null)
                        .await
                        .0,
                    StatusCode::BAD_REQUEST
                );
            }
        }
    }
    #[tokio::test]
    async fn legacy_title_series_api_keeps_web_reads_and_edits_without_merging_identities() {
        let (dir, app) = setup();
        let mut store = Store::open(&dir.path().join("catalog.sqlite")).unwrap();
        let profile = store.create_profile("Legacy web").unwrap();
        let other = store.create_profile("Other").unwrap();
        let items = [1001, 2002]
            .into_iter()
            .map(|tv| {
                let mut m = chill_catalog::scanner::parse_file(
                    std::path::Path::new("/fixture"),
                    &std::path::PathBuf::from(format!(
                        "/fixture/Show{tv}/Season 1/Doctor Who S01E01.mkv"
                    )),
                );
                m.category = "serie".into();
                m.series = Some("Doctor Who".into());
                m.season = Some(1);
                m.episode = Some(1);
                m.tmdb_id = Some(tv);
                m
            })
            .collect();
        store.snapshot(items).unwrap();
        let before = store.catalog(&profile.id).unwrap();
        store.set_watched(&profile.id, &before[0].id, true).unwrap();
        let revision = store.common_revision().unwrap();
        for version in ["2026-10-14"] {
            let endpoint = format!("/api/{version}/profiles/{}/series/Doctor%20Who", profile.id);
            let (status, view) = json_call(&app, "GET", &endpoint, Value::Null).await;
            assert_eq!(status, StatusCode::OK);
            assert_eq!(view["name"], "Doctor Who");
            assert_eq!(view["episodes"].as_array().unwrap().len(), 2);
            assert_eq!(view["counters"]["total"], 2);
            assert_eq!(view["counters"]["watched"], 1);
            assert!(view["tmdb_id"].is_null());
            assert_eq!(view["season_metadata"], json!({}));
            let (_, personal) = json_call(
                &app,
                "GET",
                &format!("/api/{version}/profiles/{}/series/Doctor%20Who", other.id),
                Value::Null,
            )
            .await;
            assert_eq!(personal["counters"]["watched"], 0);
            for (key, tv) in [
                ("tmdb:tv:1001".to_owned(), 1001),
                (
                    format!(
                        "media:{}",
                        before.iter().find(|m| m.tmdb_id == Some(2002)).unwrap().id
                    ),
                    2002,
                ),
            ] {
                let (status, one) = json_call(
                    &app,
                    "GET",
                    &format!(
                        "/api/{version}/profiles/{}/series/{key}?selector=key",
                        profile.id
                    ),
                    Value::Null,
                )
                .await;
                assert_eq!(status, StatusCode::OK);
                assert_eq!(one["episodes"].as_array().unwrap().len(), 1);
                assert_eq!(one["tmdb_id"], tv);
            }
        }
        assert_eq!(
            store.common_revision().unwrap(),
            revision,
            "GET must not change the catalog"
        );
        let endpoint = format!(
            "/api/2026-10-14/profiles/{}/series/Doctor%20Who",
            profile.id
        );
        let body = json!({"expected_revision":revision,"patch":{"name":"Doctor Who legacy","year":0,"genres":[],"overview":"Legacy batch description","cast":[],"directors":[]}});
        let (status, updated) = json_call(&app, "PUT", &endpoint, body.clone()).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(updated["episodes"].as_array().unwrap().len(), 2);
        assert_eq!(
            json_call(&app, "PUT", &endpoint, body).await.0,
            StatusCode::CONFLICT
        );
        let after = store.catalog(&profile.id).unwrap();
        for m in &before {
            let a = after.iter().find(|a| a.id == m.id).unwrap();
            assert_eq!(a.tmdb_id, m.tmdb_id);
            assert_eq!(a.path, m.path);
            assert_eq!(a.season, m.season);
            assert_eq!(a.episode, m.episode);
            assert_eq!(a.series.as_deref(), Some("Doctor Who legacy"));
        }
        assert!(
            store.series(&profile.id, "Doctor Who legacy").is_err(),
            "Internal title-based mutations must remain guarded"
        );
        let (status,updated)=json_call(&app,"PUT",&format!("/api/2026-10-14/profiles/{}/series/tmdb:tv:1001?selector=key",profile.id),json!({"expected_revision":store.common_revision().unwrap(),"patch":{"name":"Doctor Who isolated","year":0,"genres":[],"overview":"One show only","cast":[],"directors":[]}})).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(updated["episodes"].as_array().unwrap().len(), 1);
        assert_eq!(
            store.series(&profile.id, "tmdb:tv:2002").unwrap()[0]
                .series
                .as_deref(),
            Some("Doctor Who legacy")
        );
    }
    #[tokio::test]
    async fn catalog_revision_is_small_read_only_and_profile_scoped() {
        let (dir, app) = setup();
        let mut store = Store::open(&dir.path().join("catalog.sqlite")).unwrap();
        let a = store.create_profile("Revision A").unwrap();
        let b = store.create_profile("Revision B").unwrap();
        let item = chill_catalog::scanner::parse_file(
            std::path::Path::new("/fixture"),
            std::path::Path::new("/fixture/Film.mkv"),
        );
        store.snapshot(vec![item]).unwrap();
        let id = store.catalog(&a.id).unwrap()[0].id.clone();
        let endpoint = format!("/api/2026-10-14/profiles/{}/catalog/revision", a.id);
        let (status, first) = json_call(&app, "GET", &endpoint, Value::Null).await;
        assert_eq!(status, StatusCode::OK);
        assert!(first["revision"].as_str().is_some());
        assert!(first.to_string().len() < 256);
        assert_eq!(first.as_object().unwrap().len(), 1);
        store.set_watched(&b.id, &id, true).unwrap();
        assert_eq!(
            json_call(&app, "GET", &endpoint, Value::Null).await.1,
            first
        );
        store.set_rating(&a.id, &id, Some(4), "video").unwrap();
        let changed = json_call(&app, "GET", &endpoint, Value::Null).await.1;
        assert_ne!(changed, first);
        assert_eq!(
            json_call(
                &app,
                "GET",
                &format!("/api/2026-10-14/profiles/{}/catalog/revision", a.id),
                Value::Null
            )
            .await
            .1,
            changed
        );
        let sql = rusqlite::Connection::open(dir.path().join("catalog.sqlite")).unwrap();
        let generated: i64 = sql
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name LIKE 'cc_catalog_delta_checkpoint%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            generated, 0,
            "Revision GET must not materialize a checkpoint"
        );
        store.delete_profile(&a.id).unwrap();
        assert_eq!(
            json_call(&app, "GET", &endpoint, Value::Null).await.0,
            StatusCode::NOT_FOUND
        );
    }
    #[tokio::test]
    async fn catalog_changes_contract_paging_profile_isolation_and_real_delta() {
        let (dir, app) = setup();
        let (_, a) = json_call(
            &app,
            "POST",
            "/api/2026-10-14/profiles",
            json!({"name":"Delta A"}),
        )
        .await;
        let (_, b) = json_call(
            &app,
            "POST",
            "/api/2026-10-14/profiles",
            json!({"name":"Delta B"}),
        )
        .await;
        let a = a["id"].as_str().unwrap();
        let b = b["id"].as_str().unwrap();
        let mut store = Store::open(&dir.path().join("catalog.sqlite")).unwrap();
        let items: Vec<chill_catalog::model::Media> = (0..103)
            .map(|i| {
                chill_catalog::scanner::parse_file(
                    std::path::Path::new("/fixture"),
                    &std::path::Path::new("/fixture").join(format!("Film {i:03}.mkv")),
                )
            })
            .collect();
        store.snapshot(items.clone()).unwrap();
        let endpoint = format!("/api/2026-10-14/profiles/{a}/catalog/changes");
        let (status, first) = json_call(&app, "POST", &endpoint, json!({})).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(first["reset"], true);
        assert_eq!(first["total"], 103);
        assert_eq!(first["items"].as_array().unwrap().len(), 100);
        assert!(!first.to_string().contains("/fixture"));
        let snapshot = first["snapshot_id"].as_str().unwrap();
        let revision = first["revision"].as_str().unwrap();
        let id = first["items"][0]["media"]["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let page = json_call(
            &app,
            "GET",
            &format!("{endpoint}/{snapshot}?offset=100"),
            Value::Null,
        )
        .await;
        assert_eq!(page.0, StatusCode::OK);
        assert_eq!(page.1["items"].as_array().unwrap().len(), 3);
        assert_eq!(page.1["sagas"], json!([]));
        assert_eq!(page.1["series_revisions"], json!({}));
        assert_eq!(page.1["season_metadata"], json!({}));
        assert_eq!(
            json_call(
                &app,
                "GET",
                &format!("/api/2026-10-14/profiles/{b}/catalog/changes/{snapshot}?offset=100"),
                Value::Null
            )
            .await
            .0,
            StatusCode::NOT_FOUND
        );
        store.set_watched(b, &id, true).unwrap();
        let unchanged = json_call(&app, "POST", &endpoint, json!({"known_revision":revision}))
            .await
            .1;
        assert_eq!(unchanged["unchanged"], true);
        assert!(unchanged.get("items").is_none());
        assert!(unchanged.to_string().len() < 256);
        store.set_rating(a, &id, Some(4), "video").unwrap();
        let updated = json_call(&app, "POST", &endpoint, json!({"known_revision":revision}))
            .await
            .1;
        assert_eq!(updated["reset"], false);
        assert_eq!(updated["total"], 1);
        assert_eq!(updated["items"][0]["media"]["rating"], 4);
        let next_revision = updated["revision"].as_str().unwrap();
        let mut discovered = items[1..].to_vec();
        discovered.push(chill_catalog::scanner::parse_file(
            std::path::Path::new("/fixture"),
            std::path::Path::new("/fixture/Added.mkv"),
        ));
        store.snapshot(discovered).unwrap();
        store
            .forget_unavailable(std::slice::from_ref(&id), store.catalog_revision().unwrap())
            .unwrap();
        let delta = json_call(
            &app,
            "POST",
            &endpoint,
            json!({"known_revision":next_revision}),
        )
        .await
        .1;
        assert_eq!(delta["reset"], false);
        assert_eq!(delta["total"], 1);
        assert_eq!(delta["deleted_ids"], json!([id]));
        assert_eq!(delta["items"][0]["media"]["title"], "Added");
        // Completed one-page deltas do not saturate the three active slots.
        for _ in 0..4 {
            let (status, _) = json_call(
                &app,
                "POST",
                &endpoint,
                json!({"known_revision":next_revision}),
            )
            .await;
            assert_eq!(status, StatusCode::OK);
        }
        let c = store.create_profile("Delta C").unwrap();
        let d = store.create_profile("Delta D").unwrap();
        let mut active = Vec::new();
        for profile in [a, b, &c.id] {
            let route = format!("/api/2026-10-14/profiles/{profile}/catalog/changes");
            let (status, body) = json_call(&app, "POST", &route, json!({})).await;
            assert_eq!(status, StatusCode::OK);
            active.push((route, body["snapshot_id"].as_str().unwrap().to_owned()));
        }
        let (status, _) = json_call(
            &app,
            "POST",
            &format!("/api/2026-10-14/profiles/{}/catalog/changes", d.id),
            json!({}),
        )
        .await;
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
        let sql = rusqlite::Connection::open(dir.path().join("catalog.sqlite")).unwrap();
        assert_eq!(
            sql.query_row(
                "SELECT COUNT(*) FROM cc_catalog_delta_checkpoint_v2 WHERE profile_id=?1",
                [&d.id],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            0,
            "Rejected transfer must not persist a checkpoint"
        );
        for (route, transfer) in active {
            assert_eq!(
                json_call(
                    &app,
                    "GET",
                    &format!("{route}/{transfer}?offset=100"),
                    Value::Null
                )
                .await
                .0,
                StatusCode::OK,
                "Fourth admission must not evict active pages"
            );
        }
        store.delete_profile(a).unwrap();
        let sql = rusqlite::Connection::open(dir.path().join("catalog.sqlite")).unwrap();
        let checkpoints: i64 = sql
            .query_row(
                "SELECT COUNT(*) FROM cc_catalog_delta_checkpoint_v2 WHERE profile_id=?1",
                [a],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(checkpoints, 0);
    }
    fn request(method: &str, path: &str, body: Value) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(path)
            .header("host", "localhost:8443")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    }
    #[tokio::test]
    async fn dated_contract_is_canonical_and_retired_alias_is_json_404() {
        let (_dir, app) = setup();
        let prefix = crate::api_version::PREFIX;
        let response = app
            .clone()
            .oneshot(request("GET", &format!("{prefix}/server"), Value::Null))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()["x-chill-api-version"],
            crate::api_version::VERSION
        );
        let body: Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(body["api_version"], crate::api_version::VERSION);
        assert_eq!(
            body["supported_api_versions"],
            json!([crate::api_version::VERSION])
        );
        let spec = app
            .clone()
            .oneshot(request(
                "GET",
                &format!("{prefix}/openapi.json"),
                Value::Null,
            ))
            .await
            .unwrap();
        assert_eq!(spec.status(), StatusCode::OK);
        let document: Value =
            serde_json::from_slice(&spec.into_body().collect().await.unwrap().to_bytes()).unwrap();
        assert_eq!(document["info"]["version"], crate::api_version::VERSION);
        for method in ["GET", "POST", "HEAD"] {
            let response = app
                .clone()
                .oneshot(request(
                    method,
                    "/api/v1/profiles",
                    json!({"name":"retired"}),
                ))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
            assert!(
                response.headers()["content-type"]
                    .as_bytes()
                    .starts_with(b"application/json")
            );
            if method != "HEAD" {
                let body: Value = serde_json::from_slice(
                    &response.into_body().collect().await.unwrap().to_bytes(),
                )
                .unwrap();
                assert_eq!(body["code"], "ROUTE_NOT_FOUND");
            }
        }
        let created = app
            .clone()
            .oneshot(request(
                "POST",
                &format!("{}/profiles", crate::api_version::PREFIX),
                json!({"name":"Date version"}),
            ))
            .await
            .unwrap();
        assert_eq!(created.status(), StatusCode::CREATED);
        let profiles = app
            .clone()
            .oneshot(request("GET", "/api/2026-10-14/profiles", Value::Null))
            .await
            .unwrap();
        let profiles: Value =
            serde_json::from_slice(&profiles.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(profiles[0]["name"], "Date version");
        for (path, method, body, status) in [
            (
                "/profiles",
                "POST",
                json!({"unknown":true}),
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
            (
                "/profiles",
                "PUT",
                Value::Null,
                StatusCode::METHOD_NOT_ALLOWED,
            ),
            ("/unknown", "GET", Value::Null, StatusCode::NOT_FOUND),
        ] {
            let response = app
                .clone()
                .oneshot(request(
                    method,
                    &format!("{}{path}", crate::api_version::PREFIX),
                    body,
                ))
                .await
                .unwrap();
            assert_eq!(response.status(), status);
            let error: Value =
                serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                    .unwrap();
            assert!(error["code"].is_string());
            assert!(uuid::Uuid::parse_str(error["request_id"].as_str().unwrap()).is_ok());
        }
    }

    #[tokio::test]
    async fn component_update_routes_are_additive_versioned_and_closed() {
        let (_dir, app) = setup();
        for prefix in [crate::api_version::PREFIX] {
            let response = app
                .clone()
                .oneshot(request(
                    "GET",
                    &format!("{prefix}/admin/updates"),
                    Value::Null,
                ))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let status: Value =
                serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                    .unwrap();
            assert_eq!(
                status["installed"]["server"]["version"],
                env!("CARGO_PKG_VERSION")
            );
            assert_eq!(status["policies"]["server"]["hour"], 2);
            assert_eq!(status["policies"]["server"]["mode"], "automatic");
            assert_eq!(status["check_available"], true);
            for suffix in ["check", "install"] {
                let invalid = app
                    .clone()
                    .oneshot(request(
                        "POST",
                        &format!("{prefix}/admin/updates/{suffix}"),
                        json!({"component":"server","force":true}),
                    ))
                    .await
                    .unwrap();
                assert_eq!(invalid.status(), StatusCode::UNPROCESSABLE_ENTITY);
            }
        }
        let url = format!("{}/admin/updates/policy", crate::api_version::PREFIX);
        let body =
            json!({"component":"web","mode":"check","hour":3,"minute":15,"expected_revision":1});
        let saved = app
            .clone()
            .oneshot(request("PUT", &url, body.clone()))
            .await
            .unwrap();
        assert_eq!(saved.status(), StatusCode::OK);
        let stale = app
            .clone()
            .oneshot(request("POST", &url, body))
            .await
            .unwrap();
        assert_eq!(stale.status(), StatusCode::CONFLICT);
        let status = app
            .oneshot(request("GET", "/api/2026-10-14/admin/updates", Value::Null))
            .await
            .unwrap();
        let status: Value =
            serde_json::from_slice(&status.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(status["policies"]["web"]["revision"], 2);
        assert_eq!(status["policies"]["web"]["hour"], 3);
    }

    #[tokio::test]
    async fn component_checks_use_cached_receipts_without_install_helper() {
        if std::path::Path::new("/etc/chillcenter/updates.json").is_file() {
            return; // Only the legacy/no-helper deployment is exercised here.
        }
        let (dir, app) = setup();
        let store = chill_updates::StateStore::open(dir.path().join("updates/checks")).unwrap();
        for component in [
            chill_updates::Component::Server,
            chill_updates::Component::Web,
            chill_updates::Component::Themes,
        ] {
            let now = time::OffsetDateTime::now_utc();
            let op = store
                .admit_intent(component, chill_updates::IntentKind::Check, now)
                .unwrap();
            store
                .transition(
                    op.id,
                    chill_updates::OperationStatus::Pending,
                    chill_updates::OperationStatus::Running,
                    None,
                    now,
                )
                .unwrap();
            store
                .transition(
                    op.id,
                    chill_updates::OperationStatus::Running,
                    chill_updates::OperationStatus::Succeeded,
                    Some("NO_UPDATE"),
                    now,
                )
                .unwrap();
            let response = app
                .clone()
                .oneshot(request(
                    "POST",
                    &format!("{}/admin/updates/check", crate::api_version::PREFIX),
                    json!({"component":component}),
                ))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let body: Value =
                serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                    .unwrap();
            assert_eq!(body["cached"], true);
            assert_eq!(body["operation"]["result_code"], "NO_UPDATE");
            let response = app
                .clone()
                .oneshot(request(
                    "POST",
                    &format!("{}/admin/updates/install", crate::api_version::PREFIX),
                    json!({"component":component}),
                ))
                .await
                .unwrap();
            assert_eq!(
                response.status(),
                StatusCode::SERVICE_UNAVAILABLE,
                "Checking cannot grant installation rights"
            );
        }
        assert!(
            !dir.path().join("updates/available.json").exists(),
            "Passive discovery never writes privileged-helper hints"
        );
    }
    #[tokio::test]
    async fn cancelled_queued_and_started_mutations_never_commit_late() {
        for started in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let actor = DatabaseActor::open(dir.path().join("test.sqlite")).unwrap();
            let (entered_tx, entered_rx) = oneshot::channel();
            let (release_tx, release_rx) = std::sync::mpsc::channel();
            let worker = actor.clone();
            let task = tokio::spawn(async move {
                worker
                    .call_with_budget(Duration::from_millis(100), move |store| {
                        let _ = entered_tx.send(());
                        release_rx.recv().unwrap();
                        if started {
                            store.create_profile("Must not appear")?;
                        }
                        Ok(())
                    })
                    .await
            });
            entered_rx.await.unwrap();
            if !started {
                let error = actor
                    .call_with_budget(Duration::from_millis(10), |store| {
                        store.create_profile("Queued deletion/retry hazard")
                    })
                    .await
                    .unwrap_err();
                assert_eq!(error.1, "REQUEST_TIMEOUT");
            } else {
                let error = task.await.unwrap().unwrap_err();
                assert_eq!(error.1, "REQUEST_TIMEOUT");
                release_tx.send(()).unwrap();
                assert!(
                    actor
                        .call(|store| store.profiles())
                        .await
                        .unwrap()
                        .is_empty()
                );
                continue;
            }
            release_tx.send(()).unwrap();
            task.await.unwrap().unwrap();
            assert!(
                actor
                    .call(|store| store.profiles())
                    .await
                    .unwrap()
                    .is_empty()
            );
        }
    }
    #[tokio::test]
    async fn typed_errors_distinguish_configuration_database_and_invalid_values() {
        let (_dir, app) = setup();
        let response = app
            .oneshot(request("POST", "/api/2026-10-14/admin/scan", json!({})))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body: Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(body["code"], "ROOT_REQUIRED");
        assert_eq!(
            ApiError::from(chill_domain::StoreFailure::DatabaseOffline).0,
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(
            ApiError::from(chill_domain::StoreFailure::InvalidOperation).0,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    #[tokio::test]
    async fn interactive_reads_remain_responsive_during_large_snapshot_write() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("catalog.sqlite");
        let actor = DatabaseActor::open(path.clone()).unwrap();
        actor
            .call(|store| store.create_profile("Available during scan"))
            .await
            .unwrap();
        let (entered_tx, entered_rx) = oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let writer = tokio::task::spawn_blocking(move || {
            let mut database = chill_catalog::database::Database::open(&path).unwrap();
            let media = (0..2_000)
                .map(|n| {
                    chill_catalog::scanner::parse_file(
                        std::path::Path::new("/nas"),
                        &std::path::PathBuf::from(format!("/nas/Film{n}.mp4")),
                    )
                })
                .collect();
            database
                .snapshot_with(media, |_, _| {
                    let _ = entered_tx.send(());
                    release_rx.recv().unwrap();
                    Ok(())
                })
                .unwrap();
        });
        entered_rx.await.unwrap();
        let response =
            tokio::time::timeout(Duration::from_secs(1), actor.call(|store| store.profiles()))
                .await;
        release_tx.send(()).unwrap();
        assert_eq!(response.unwrap().unwrap().len(), 1);
        writer.await.unwrap();
    }
    #[tokio::test]
    async fn startup_recovers_interrupted_fingerprints_without_generic_open_mutating_jobs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("catalog.sqlite");
        let mut store = Store::open(&path).unwrap();
        let item = chill_catalog::scanner::parse_file(
            std::path::Path::new("/nas"),
            std::path::Path::new("/nas/recovery.mp4"),
        );
        store.snapshot(vec![item]).unwrap();
        let profile = store.create_profile("Recovery").unwrap();
        let id = store.catalog(&profile.id).unwrap()[0].id.clone();
        store
            .mark_fingerprint_queued(&id, chill_domain::fingerprint::FingerprintKind::Full)
            .unwrap();
        store
            .mark_fingerprint_running(&id, store.media_revision(&id).unwrap())
            .unwrap();
        drop(store);
        let reopened = Store::open(&path).unwrap();
        assert_eq!(reopened.fingerprint_status(&id).unwrap().state, "running");
        drop(reopened);
        let actor = DatabaseActor::open(path).unwrap();
        let status = actor
            .call(move |store| {
                let status = store.fingerprint_status(&id)?;
                assert_eq!(store.fingerprint_scan_targets_after("", 16)?.len(), 1);
                store.mark_fingerprint_queued(
                    &id,
                    chill_domain::fingerprint::FingerprintKind::Full,
                )?;
                Ok(status)
            })
            .await
            .unwrap();
        assert_eq!(status.state, "failed");
    }
    #[tokio::test]
    async fn configurable_root_is_persisted_through_admin_api() {
        let (dir, app) = setup();
        let response = app
            .clone()
            .oneshot(request(
                "PUT",
                "/api/2026-10-14/admin/library",
                json!({"video_root":"/mnt/custom-movies","expected_revision":0}),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let data: Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(data["video_root"], "/mnt/custom-movies");
        assert_eq!(data["revision"], 1);
        assert!(dir.path().join("library.json").is_file());
        assert_eq!(
            app.oneshot(request(
                "PUT",
                "/api/2026-10-14/admin/library",
                json!({"video_root":"/mnt/stale","expected_revision":0})
            ))
            .await
            .unwrap()
            .status(),
            StatusCode::CONFLICT
        );
    }

    #[tokio::test]
    async fn dated_and_v1_library_updates_keep_extra_locations_when_legacy_body_omits_them() {
        let (_dir, app) = setup();
        let first = app
            .clone()
            .oneshot(request(
                "PUT",
                "/api/2026-10-14/admin/library",
                json!({"video_root":"/mnt/films","additional_video_roots":["/mnt/series"],"expected_revision":0}),
            ))
            .await
            .unwrap();
        assert_eq!(first.status(), StatusCode::OK);
        let legacy = app
            .clone()
            .oneshot(request(
                "PUT",
                "/api/2026-10-14/admin/library",
                json!({"video_root":"/mnt/films-renamed","expected_revision":1}),
            ))
            .await
            .unwrap();
        assert_eq!(legacy.status(), StatusCode::OK);
        let legacy_body: Value =
            serde_json::from_slice(&legacy.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(
            legacy_body["additional_video_roots"],
            json!(["/mnt/series"])
        );
        for prefix in ["/api/2026-10-14"] {
            let response = app
                .clone()
                .oneshot(request(
                    "GET",
                    &format!("{prefix}/admin/library"),
                    Value::Null,
                ))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let body: Value =
                serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                    .unwrap();
            assert_eq!(body["additional_video_roots"], json!(["/mnt/series"]));
        }
    }
    #[tokio::test]
    async fn zero_profiles_is_bootstrappable() {
        let (_dir, app) = setup();
        let empty = app
            .clone()
            .oneshot(request("GET", "/api/2026-10-14/profiles", Value::Null))
            .await
            .unwrap();
        assert_eq!(empty.status(), StatusCode::OK);
        let first = app
            .oneshot(request(
                "POST",
                "/api/2026-10-14/profiles",
                json!({"name":"Premier"}),
            ))
            .await
            .unwrap();
        assert_eq!(first.status(), StatusCode::CREATED);
    }
    #[tokio::test]
    async fn browser_and_dns_rebinding_requests_are_refused() {
        let (_dir, app) = setup();
        let mut browser = request("PUT", "/api/2026-10-14/admin/library", json!({}));
        browser
            .headers_mut()
            .insert("origin", "https://other.example".parse().unwrap());
        assert_eq!(
            app.clone().oneshot(browser).await.unwrap().status(),
            StatusCode::FORBIDDEN
        );
        let mut rebound = request("GET", "/api/2026-10-14/server", Value::Null);
        rebound
            .headers_mut()
            .insert("host", "attacker.example:8443".parse().unwrap());
        assert_eq!(
            app.oneshot(rebound).await.unwrap().status(),
            StatusCode::FORBIDDEN
        );
    }

    #[tokio::test]
    async fn plain_http_listener_accepts_only_its_own_origin_and_ports() {
        let (_dir, app) = setup_with(true);
        let call = |host: &str, headers: &[(&str, &str)]| {
            let mut r = request("PUT", "/api/2026-10-14/admin/library", json!({}));
            r.headers_mut().insert("host", host.parse().unwrap());
            for (k, v) in headers {
                r.headers_mut().insert(
                    axum::http::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                    v.parse().unwrap(),
                );
            }
            let app = app.clone();
            async move { app.oneshot(r).await.unwrap().status() }
        };
        let refused = StatusCode::FORBIDDEN;
        // Hosts: the HTTP listener's own port and the default port 80; never the HTTPS port or another one.
        for host in ["localhost:8080", "localhost", "localhost:80"] {
            assert_ne!(call(host, &[]).await, refused, "{host}");
        }
        for host in [
            "localhost:8443",
            "localhost:443",
            "localhost:9090",
            "attacker.example:8080",
            "attacker.example",
        ] {
            assert_eq!(call(host, &[]).await, refused, "{host}");
        }
        // Browsers send no Sec-Fetch-* to a plain-HTTP LAN origin, but always an Origin on writes: it must be our own.
        assert_ne!(
            call("localhost:8080", &[("origin", "http://localhost:8080")]).await,
            refused
        );
        assert_ne!(
            call("localhost", &[("origin", "http://localhost")]).await,
            refused
        );
        assert_ne!(
            call("localhost", &[("origin", "HTTP://LOCALHOST")]).await,
            refused
        );
        for origin in [
            "http://attacker.example",
            "http://localhost:9090",
            "https://localhost:8080",
            "null",
            "",
        ] {
            assert_eq!(
                call("localhost:8080", &[("origin", origin)]).await,
                refused,
                "origin {origin:?}"
            );
        }
        // Cross-site and same-site hints are refused when a browser does send them (localhost is trustworthy).
        for site in ["cross-site", "same-site"] {
            assert_eq!(
                call("localhost:8080", &[("sec-fetch-site", site)]).await,
                refused
            );
        }
        assert_ne!(
            call(
                "localhost:8080",
                &[
                    ("sec-fetch-site", "same-origin"),
                    ("origin", "http://localhost:8080")
                ]
            )
            .await,
            refused
        );
        assert_eq!(
            call(
                "localhost:8080",
                &[
                    ("sec-fetch-site", "same-origin"),
                    ("origin", "https://localhost:8080")
                ]
            )
            .await,
            refused
        );
    }

    #[tokio::test]
    async fn secrets_and_archives_are_https_only() {
        let (_dir, http) = setup_with(true);
        let (_dir2, https) = setup();
        let get = |path: &str, host: &str| {
            let mut r = request("GET", path, Value::Null);
            r.headers_mut().insert("host", host.parse().unwrap());
            r
        };
        for path in [
            "/api/2026-10-14/admin/tmdb",
            "/api/2026-10-14/admin/tmdb/jobs",
            "/api/2026-10-14/admin/tmdb/jobs/x/cancel",
            "/api/2026-10-14/admin/tmdb",
            "/api/2026-10-14/admin/exports",
            "/api/2026-10-14/admin/exports/abc",
            "/api/2026-10-14/admin/imports",
            "/api/2026-10-14/admin/imports/abc/apply",
        ] {
            assert_eq!(
                http.clone()
                    .oneshot(get(path, "localhost:8080"))
                    .await
                    .unwrap()
                    .status(),
                StatusCode::FORBIDDEN,
                "{path} must be refused over HTTP"
            );
            assert_ne!(
                https
                    .clone()
                    .oneshot(get(path, "localhost:8443"))
                    .await
                    .unwrap()
                    .status(),
                StatusCode::FORBIDDEN,
                "{path} must stay available over HTTPS"
            );
        }
        // Everything the web client needs stays reachable; look-alike prefixes are not caught.
        for path in [
            "/api/2026-10-14/admin/library",
            "/api/2026-10-14/profiles",
            "/api/2026-10-14/admin/scan",
            "/api/2026-10-14/admin/tmdbx",
            "/api/2026-10-14/admin/exportsx",
        ] {
            assert_ne!(
                http.clone()
                    .oneshot(get(path, "localhost:8080"))
                    .await
                    .unwrap()
                    .status(),
                StatusCode::FORBIDDEN,
                "{path}"
            );
        }
        assert!(super::https_only_path("/api/2026-10-14//admin/tmdb"));
        assert!(!super::https_only_path("/admin/tmdb"));
    }

    #[tokio::test]
    async fn https_listener_still_refuses_an_origin_without_fetch_metadata() {
        let (_dir, app) = setup();
        for origin in ["https://localhost:8443", "http://localhost:8443"] {
            let mut r = request("PUT", "/api/2026-10-14/admin/library", json!({}));
            r.headers_mut().insert("origin", origin.parse().unwrap());
            assert_eq!(
                app.clone().oneshot(r).await.unwrap().status(),
                StatusCode::FORBIDDEN,
                "{origin}"
            );
        }
    }

    #[tokio::test]
    async fn default_https_port_is_accepted_for_a_redirected_443_but_names_and_other_ports_are_not()
    {
        let (_dir, app) = setup();
        let with_host = |host: &str, origin: Option<&str>| {
            let mut r = request("GET", "/api/2026-10-14/server", Value::Null);
            r.headers_mut().insert("host", host.parse().unwrap());
            if let Some(origin) = origin {
                r.headers_mut()
                    .insert("sec-fetch-site", "same-origin".parse().unwrap());
                r.headers_mut().insert("origin", origin.parse().unwrap());
            }
            r
        };
        let status = |r: Request<Body>| {
            let app = app.clone();
            async move { app.oneshot(r).await.unwrap().status() }
        };
        assert_eq!(
            status(with_host("localhost:8443", None)).await,
            StatusCode::OK
        );
        assert_eq!(status(with_host("localhost", None)).await, StatusCode::OK);
        assert_eq!(
            status(with_host("localhost:443", None)).await,
            StatusCode::OK
        );
        assert_eq!(
            status(with_host("localhost", Some("https://localhost"))).await,
            StatusCode::OK
        );
        // An Origin without the port does not match a Host that carries one, and vice versa.
        assert_eq!(
            status(with_host("localhost:8443", Some("https://localhost"))).await,
            StatusCode::FORBIDDEN
        );
        for refused in [
            "localhost:9443",
            "localhost:80",
            "attacker.example",
            "attacker.example:443",
        ] {
            assert_eq!(
                status(with_host(refused, None)).await,
                StatusCode::FORBIDDEN,
                "{refused}"
            );
        }
    }

    #[tokio::test]
    async fn same_origin_browser_is_accepted_and_cross_site_is_refused() {
        let (dir, app) = setup();
        let with = |site: &str, origin: Option<&str>, path: &str| {
            let mut r = request("GET", path, Value::Null);
            r.headers_mut()
                .insert("sec-fetch-site", site.parse().unwrap());
            if let Some(origin) = origin {
                r.headers_mut().insert("origin", origin.parse().unwrap());
            }
            r
        };
        let status = |r: Request<Body>| {
            let app = app.clone();
            async move { app.oneshot(r).await.unwrap().status() }
        };
        assert_eq!(
            status(with("same-origin", None, "/api/2026-10-14/server")).await,
            StatusCode::OK
        );
        assert_eq!(
            status(with(
                "same-origin",
                Some("https://localhost:8443"),
                "/api/2026-10-14/server"
            ))
            .await,
            StatusCode::OK
        );
        for refused in [
            with("cross-site", None, "/api/2026-10-14/server"),
            with("same-site", None, "/api/2026-10-14/server"),
            with("none", None, "/api/2026-10-14/server"),
            with(
                "same-origin",
                Some("https://other.example"),
                "/api/2026-10-14/server",
            ),
        ] {
            assert_eq!(status(refused).await, StatusCode::FORBIDDEN);
        }
        // Without a web directory the static fallback is a plain 404; with one it serves.
        assert_eq!(status(with("none", None, "/")).await, StatusCode::NOT_FOUND);
        std::fs::create_dir(dir.path().join("web")).unwrap();
        std::fs::write(dir.path().join("web/index.html"), b"<!doctype html>").unwrap();
        crate::web_static::set_test_root(Some(dir.path().join("web")));
        assert_eq!(status(with("none", None, "/")).await, StatusCode::OK);
        assert_eq!(
            status(with("none", None, "/.env")).await,
            StatusCode::NOT_FOUND
        );
    }

    // The test web root lives in a thread-local: `#[tokio::test]` runs the whole request on the test's own thread.
    #[tokio::test]
    async fn static_files_obey_the_same_guard_and_unknown_api_routes_stay_json_404() {
        let (dir, app) = setup();
        std::fs::create_dir(dir.path().join("web")).unwrap();
        std::fs::write(dir.path().join("web/index.html"), b"<!doctype html>").unwrap();
        crate::web_static::set_test_root(Some(dir.path().join("web")));
        let send = |method: &str, path: &str, headers: Vec<(&str, Vec<u8>)>| {
            let app = app.clone();
            let mut r = Request::builder()
                .method(method)
                .uri(path)
                .header("host", "localhost:8443")
                .body(Body::empty())
                .unwrap();
            for (name, value) in headers {
                r.headers_mut().insert(
                    axum::http::HeaderName::from_bytes(name.as_bytes()).unwrap(),
                    axum::http::HeaderValue::from_bytes(&value).unwrap(),
                );
            }
            async move { app.oneshot(r).await.unwrap() }
        };
        let same = || vec![("sec-fetch-site", b"same-origin".to_vec())];
        // Static files are refused to other sites, like the API.
        assert_eq!(
            send("GET", "/", vec![("sec-fetch-site", b"cross-site".to_vec())])
                .await
                .status(),
            StatusCode::FORBIDDEN
        );
        // Typed-in navigation (`none`) may only GET static files.
        assert_eq!(
            send("POST", "/", vec![("sec-fetch-site", b"none".to_vec())])
                .await
                .status(),
            StatusCode::FORBIDDEN
        );
        // Origin `null`, non-UTF-8 origin, foreign origin: refused even when same-origin is claimed.
        for origin in [
            b"null".to_vec(),
            vec![0xff, 0xfe],
            b"https://other.example".to_vec(),
        ] {
            let mut h = same();
            h.push(("origin", origin));
            assert_eq!(send("GET", "/", h).await.status(), StatusCode::FORBIDDEN);
        }
        // Scheme/host case is irrelevant to the origin comparison.
        let mut h = same();
        h.push(("origin", b"HTTPS://LOCALHOST:8443".to_vec()));
        assert_eq!(send("GET", "/", h).await.status(), StatusCode::OK);
        // HEAD is served without a body, other methods are refused on static files.
        let head = send("HEAD", "/index.html", same()).await;
        assert_eq!(head.status(), StatusCode::OK);
        assert!(
            head.into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .is_empty()
        );
        assert_eq!(
            send("POST", "/index.html", same()).await.status(),
            StatusCode::METHOD_NOT_ALLOWED
        );
        // Unknown API routes keep the JSON ROUTE_NOT_FOUND envelope for every method.
        for method in ["GET", "POST", "PUT", "DELETE"] {
            let response = send(method, "/api/2026-10-14/does-not-exist", Vec::new()).await;
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{method}");
            let body: Value =
                serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                    .unwrap();
            assert_eq!(body["code"], "ROUTE_NOT_FOUND", "{method}");
        }
    }

    #[tokio::test]
    async fn http2_authority_without_host_is_accepted() {
        let (_dir, app) = setup();
        let request = Request::builder()
            .version(axum::http::Version::HTTP_2)
            .uri("https://localhost:8443/api/2026-10-14/server")
            .body(Body::empty())
            .unwrap();
        assert_eq!(app.oneshot(request).await.unwrap().status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn profile_rename_requires_current_revision_and_missing_profiles_are_404() {
        let (_dir, app) = setup();
        let response = app
            .clone()
            .oneshot(request(
                "POST",
                "/api/2026-10-14/profiles",
                json!({"name":"A"}),
            ))
            .await
            .unwrap();
        let p: Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        let path = format!("/api/2026-10-14/profiles/{}", p["id"].as_str().unwrap());
        assert_eq!(
            app.clone()
                .oneshot(request(
                    "PUT",
                    &path,
                    json!({"name":"B","expected_revision":1})
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
        assert_eq!(
            app.clone()
                .oneshot(request(
                    "PUT",
                    &path,
                    json!({"name":"C","expected_revision":1})
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            app.oneshot(request(
                "GET",
                "/api/2026-10-14/profiles/missing/preferences",
                Value::Null
            ))
            .await
            .unwrap()
            .status(),
            StatusCode::NOT_FOUND
        );
    }

    #[tokio::test]
    async fn http_catalog_queries_are_paged_and_personal_state_is_isolated() {
        let dir = tempfile::tempdir().unwrap();
        let config = ServerConfig {
            data_dir: dir.path().to_owned(),
            ..ServerConfig::default()
        };
        let certificate = crate::tls::ensure(&config, time::OffsetDateTime::now_utc()).unwrap();
        let database = DatabaseActor::open(dir.path().join("test.sqlite")).unwrap();
        let (a, b) = database
            .call(|store| {
                let a = store.create_profile("A")?;
                let b = store.create_profile("B")?;
                let mut media: Vec<_> = (0..51)
                    .map(|n| {
                        chill_catalog::scanner::parse_file(
                            std::path::Path::new("/nas"),
                            &std::path::PathBuf::from(format!("/nas/Film{n:03}.2020.mp4")),
                        )
                    })
                    .collect();
                for name in ["Series.S01E01.Needle.mkv", "Series.S01E02.Different.mkv"] {
                    media.push(chill_catalog::scanner::parse_file(
                        std::path::Path::new("/nas"),
                        &std::path::PathBuf::from(format!("/nas/{name}")),
                    ));
                }
                store.snapshot(media)?;
                Ok((a.id, b.id))
            })
            .await
            .unwrap();
        let state = AppState {
            playback: Default::default(),
            tmdb_jobs: Arc::new(crate::tmdb_api::TmdbJobService::default()),
            database,
            config: Arc::new(config),
            library: Arc::new(Mutex::new(LibrarySettings::default())),
            certificate: Arc::new(Mutex::new(certificate.schedule)),
            scan: Arc::new(Mutex::new(crate::catalog_api::ScanStatus::default())),
        };
        let app = router(state);
        for (query, expected_count, expected_title) in [
            ("category=film&q=Film000", 1, Some("Film000")),
            ("category=serie&q=Needle", 1, Some("Needle")),
            ("category=film&q=Needle", 0, None),
            ("category=serie&q=DoesNotExist", 0, None),
        ] {
            let response = app
                .clone()
                .oneshot(request(
                    "GET",
                    &format!("/api/2026-10-14/profiles/{a}/catalog?{query}"),
                    Value::Null,
                ))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let page: Value =
                serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                    .unwrap();
            assert_eq!(page["items"].as_array().unwrap().len(), expected_count);
            if let Some(title) = expected_title {
                assert!(page["items"][0]["title"].as_str().unwrap().contains(title));
            }
        }
        for (method, path, body, code) in [
            (
                "PUT",
                format!("/api/2026-10-14/profiles/{a}/media/{b}/watched"),
                json!({"watched":true}),
                "MEDIA_NOT_FOUND",
            ),
            (
                "PUT",
                format!("/api/2026-10-14/profiles/{a}/media/{b}/rating"),
                json!({"rating":5}),
                "MEDIA_NOT_FOUND",
            ),
            (
                "GET",
                format!("/api/2026-10-14/profiles/{}/catalog", uuid::Uuid::new_v4()),
                Value::Null,
                "PROFILE_NOT_FOUND",
            ),
        ] {
            let response = app
                .clone()
                .oneshot(request(method, &path, body))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
            let body: Value =
                serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                    .unwrap();
            assert_eq!(body["code"], code);
        }
        let page = app
            .clone()
            .oneshot(request(
                "GET",
                &format!(
                    "/api/2026-10-14/profiles/{a}/catalog?limit=50&offset=0&category=film&q=Film"
                ),
                Value::Null,
            ))
            .await
            .unwrap();
        assert_eq!(page.status(), StatusCode::OK);
        let page: Value =
            serde_json::from_slice(&page.into_body().collect().await.unwrap().to_bytes()).unwrap();
        assert_eq!(page["items"].as_array().unwrap().len(), 50);
        assert_eq!(page["has_more"], true);
        assert!(!page["items"][0]["path"].as_str().unwrap().starts_with('/'));
        let id = page["items"][0]["id"].as_str().unwrap();
        for (method, path, body) in [
            (
                "GET",
                format!("/api/2026-10-14/profiles/{id}/catalog"),
                Value::Null,
            ),
            (
                "PUT",
                format!("/api/2026-10-14/profiles/{id}/media/{id}/watched"),
                json!({"watched":true}),
            ),
            (
                "PUT",
                format!("/api/2026-10-14/profiles/{id}/media/{id}/rating"),
                json!({"rating":5}),
            ),
        ] {
            let response = app
                .clone()
                .oneshot(request(method, &path, body))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
            let body: Value =
                serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                    .unwrap();
            assert_eq!(body["code"], "PROFILE_NOT_FOUND");
        }
        assert_eq!(
            app.clone()
                .oneshot(request(
                    "PUT",
                    &format!("/api/2026-10-14/profiles/{a}/media/{id}/watched"),
                    json!({"watched":true})
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
        let other = app
            .clone()
            .oneshot(request(
                "GET",
                &format!("/api/2026-10-14/profiles/{b}/catalog?limit=50&offset=0"),
                Value::Null,
            ))
            .await
            .unwrap();
        let other: Value =
            serde_json::from_slice(&other.into_body().collect().await.unwrap().to_bytes()).unwrap();
        assert_eq!(other["items"][0]["id"], id);
        assert_eq!(other["items"][0]["watched"], false);
        let last = app
            .clone()
            .oneshot(request(
                "GET",
                &format!("/api/2026-10-14/profiles/{b}/catalog?limit=50&offset=50&category=film"),
                Value::Null,
            ))
            .await
            .unwrap();
        let last: Value =
            serde_json::from_slice(&last.into_body().collect().await.unwrap().to_bytes()).unwrap();
        assert_eq!(last["items"].as_array().unwrap().len(), 1);
        assert_eq!(last["has_more"], false);
        let mut delete = request(
            "DELETE",
            &format!("/api/2026-10-14/profiles/{a}"),
            Value::Null,
        );
        delete
            .headers_mut()
            .insert("x-chill-confirm-delete", a.parse().unwrap());
        assert_eq!(
            app.clone().oneshot(delete).await.unwrap().status(),
            StatusCode::OK
        );
        for (method, path, body) in [
            (
                "GET",
                format!("/api/2026-10-14/profiles/{a}/catalog"),
                Value::Null,
            ),
            (
                "PUT",
                format!("/api/2026-10-14/profiles/{a}/media/{id}/watched"),
                json!({"watched":true}),
            ),
            (
                "PUT",
                format!("/api/2026-10-14/profiles/{a}/media/{id}/rating"),
                json!({"rating":5}),
            ),
            (
                "PUT",
                format!("/api/2026-10-14/profiles/{a}/preferences"),
                json!({"audio_language":"fr","subtitle_language":"fr","autoplay_seconds":10}),
            ),
        ] {
            let response = app
                .clone()
                .oneshot(request(method, &path, body))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
            let body: Value =
                serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                    .unwrap();
            assert_eq!(body["code"], "PROFILE_NOT_FOUND");
        }
        // Inject only a temporary database read failure after successful personal
        // operations. It must not masquerade as a removed media item/404.
        let connection = rusqlite::Connection::open(dir.path().join("test.sqlite")).unwrap();
        connection.execute_batch("DROP TABLE media;").unwrap();
        let response = app
            .clone()
            .oneshot(request(
                "PUT",
                &format!("/api/2026-10-14/profiles/{b}/media/{id}/rating"),
                json!({"rating":5}),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body: Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(body["code"], "DATABASE_OFFLINE");
    }
}
