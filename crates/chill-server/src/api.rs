//! Bounded SQLite actor: no database call blocks either of the two Tokio workers.
use crate::{
    config::{LibrarySettings, ServerConfig},
    tls::CertificateSchedule,
};
use axum::{
    Json, Router,
    body::Body,
    extract::{OriginalUri, Path, State},
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
}
struct Cancellation(Arc<AtomicBool>);
impl Drop for Cancellation {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}
impl DatabaseActor {
    pub fn open(path: PathBuf) -> Result<Self, String> {
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
        Ok(Self { sender })
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
        .merge(crate::identity_api::routes())
        .merge(crate::tmdb_api::routes())
        .merge(crate::playback::routes())
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

pub fn router(state: AppState) -> Router {
    let api = api_routes();
    Router::new()
        .nest(crate::api_version::PREFIX, api.clone())
        .nest("/api/v1", api)
        .fallback(crate::web_static::serve)
        .layer(axum::extract::DefaultBodyLimit::max(1024 * 1024))
        .layer(middleware::from_fn_with_state(state.clone(), same_origin))
        .layer(middleware::from_fn(crate::api_version::normalize_response))
        .with_state(state)
}

/// No CORS. Native Rust requests have no Origin. A browser request is accepted
/// only when it is same-origin with this server (the bundled web client, ADR-0012):
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
    let header = |name: &str| request.headers().get(name).map(|v| v.to_str().ok());
    let browser_allowed = match (header("sec-fetch-site"), header("origin")) {
        (None, None) => true,
        (Some(Some("same-origin")), None) => true,
        (Some(Some("same-origin")), Some(Some(origin))) => {
            origin.eq_ignore_ascii_case(&format!("https://{host}"))
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
    if !state
        .config
        .tls_names
        .iter()
        .any(|n| n.eq_ignore_ascii_case(raw_host))
        || authority.port_u16().unwrap_or(443) != state.config.listen.port()
    {
        return Err(ApiError(StatusCode::FORBIDDEN, "HOST_REFUSED"));
    }
    Ok(())
}

async fn server(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<Value>, ApiError> {
    let certificate = state
        .certificate
        .lock()
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "BUSY"))?
        .clone();
    Ok(Json(
        json!({"server_id":certificate.server_id,"api_version":if uri.path().starts_with("/api/v1/"){json!(1)}else{json!(crate::api_version::VERSION)},"supported_api_versions":[crate::api_version::VERSION,"v1"],"version":env!("CARGO_PKG_VERSION"),"capabilities":["profiles","configurable_library","catalog_pages","manual_personal_state","scan","playback_leases","media_range","progress","tmdb_configuration","tmdb_jobs","ai_filters","openapi"],"tls":{"renew_at":certificate.renew_at,"expires_at":certificate.expires_at}}),
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
        let result = settings.update(&state.config, body.video_root, body.expected_revision);
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
                .call(move |store| store.create_profile(&body.name))
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
    fn setup() -> (tempfile::TempDir, Router) {
        let dir = tempfile::tempdir().unwrap();
        let config = ServerConfig {
            data_dir: dir.path().to_owned(),
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
        (dir, router(state))
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
    async fn dated_contract_and_legacy_alias_share_data_and_errors() {
        let (_dir, app) = setup();
        for (prefix, version) in [
            (
                crate::api_version::PREFIX,
                json!(crate::api_version::VERSION),
            ),
            ("/api/v1", json!(1)),
        ] {
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
            assert_eq!(body["api_version"], version);
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
                serde_json::from_slice(&spec.into_body().collect().await.unwrap().to_bytes())
                    .unwrap();
            assert_eq!(document["info"]["version"], crate::api_version::VERSION);
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
            .oneshot(request("GET", "/api/v1/profiles", Value::Null))
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
            .oneshot(request("POST", "/api/v1/admin/scan", json!({})))
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
                "/api/v1/admin/library",
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
                "/api/v1/admin/library",
                json!({"video_root":"/mnt/stale","expected_revision":0})
            ))
            .await
            .unwrap()
            .status(),
            StatusCode::CONFLICT
        );
    }
    #[tokio::test]
    async fn zero_profiles_is_bootstrappable() {
        let (_dir, app) = setup();
        let empty = app
            .clone()
            .oneshot(request("GET", "/api/v1/profiles", Value::Null))
            .await
            .unwrap();
        assert_eq!(empty.status(), StatusCode::OK);
        let first = app
            .oneshot(request(
                "POST",
                "/api/v1/profiles",
                json!({"name":"Premier"}),
            ))
            .await
            .unwrap();
        assert_eq!(first.status(), StatusCode::CREATED);
    }
    #[tokio::test]
    async fn browser_and_dns_rebinding_requests_are_refused() {
        let (_dir, app) = setup();
        let mut browser = request("PUT", "/api/v1/admin/library", json!({}));
        browser
            .headers_mut()
            .insert("origin", "https://other.example".parse().unwrap());
        assert_eq!(
            app.clone().oneshot(browser).await.unwrap().status(),
            StatusCode::FORBIDDEN
        );
        let mut rebound = request("GET", "/api/v1/server", Value::Null);
        rebound
            .headers_mut()
            .insert("host", "attacker.example:8443".parse().unwrap());
        assert_eq!(
            app.oneshot(rebound).await.unwrap().status(),
            StatusCode::FORBIDDEN
        );
    }

    #[tokio::test]
    async fn same_origin_browser_is_accepted_and_cross_site_is_refused() {
        let (dir, app) = setup();
        let with = |site: &str, origin: Option<&str>, path: &str| {
            let mut r = request("GET", path, Value::Null);
            r.headers_mut().insert("sec-fetch-site", site.parse().unwrap());
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
            status(with("same-origin", None, "/api/v1/server")).await,
            StatusCode::OK
        );
        assert_eq!(
            status(with(
                "same-origin",
                Some("https://localhost:8443"),
                "/api/v1/server"
            ))
            .await,
            StatusCode::OK
        );
        for refused in [
            with("cross-site", None, "/api/v1/server"),
            with("same-site", None, "/api/v1/server"),
            with("none", None, "/api/v1/server"),
            with("same-origin", Some("https://other.example"), "/api/v1/server"),
        ] {
            assert_eq!(status(refused).await, StatusCode::FORBIDDEN);
        }
        // Without a web directory the static fallback is a plain 404; with one it serves.
        assert_eq!(
            status(with("none", None, "/")).await,
            StatusCode::NOT_FOUND
        );
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
        for origin in [b"null".to_vec(), vec![0xff, 0xfe], b"https://other.example".to_vec()] {
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
        assert!(head.into_body().collect().await.unwrap().to_bytes().is_empty());
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
            .uri("https://localhost:8443/api/v1/server")
            .body(Body::empty())
            .unwrap();
        assert_eq!(app.oneshot(request).await.unwrap().status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn profile_rename_requires_current_revision_and_missing_profiles_are_404() {
        let (_dir, app) = setup();
        let response = app
            .clone()
            .oneshot(request("POST", "/api/v1/profiles", json!({"name":"A"})))
            .await
            .unwrap();
        let p: Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        let path = format!("/api/v1/profiles/{}", p["id"].as_str().unwrap());
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
                "/api/v1/profiles/missing/preferences",
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
                    &format!("/api/v1/profiles/{a}/catalog?{query}"),
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
                format!("/api/v1/profiles/{a}/media/{b}/watched"),
                json!({"watched":true}),
                "MEDIA_NOT_FOUND",
            ),
            (
                "PUT",
                format!("/api/v1/profiles/{a}/media/{b}/rating"),
                json!({"rating":5}),
                "MEDIA_NOT_FOUND",
            ),
            (
                "GET",
                format!("/api/v1/profiles/{}/catalog", uuid::Uuid::new_v4()),
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
                &format!("/api/v1/profiles/{a}/catalog?limit=50&offset=0&category=film&q=Film"),
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
            ("GET", format!("/api/v1/profiles/{id}/catalog"), Value::Null),
            (
                "PUT",
                format!("/api/v1/profiles/{id}/media/{id}/watched"),
                json!({"watched":true}),
            ),
            (
                "PUT",
                format!("/api/v1/profiles/{id}/media/{id}/rating"),
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
                    &format!("/api/v1/profiles/{a}/media/{id}/watched"),
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
                &format!("/api/v1/profiles/{b}/catalog?limit=50&offset=0"),
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
                &format!("/api/v1/profiles/{b}/catalog?limit=50&offset=50&category=film"),
                Value::Null,
            ))
            .await
            .unwrap();
        let last: Value =
            serde_json::from_slice(&last.into_body().collect().await.unwrap().to_bytes()).unwrap();
        assert_eq!(last["items"].as_array().unwrap().len(), 1);
        assert_eq!(last["has_more"], false);
        let mut delete = request("DELETE", &format!("/api/v1/profiles/{a}"), Value::Null);
        delete
            .headers_mut()
            .insert("x-chill-confirm-delete", a.parse().unwrap());
        assert_eq!(
            app.clone().oneshot(delete).await.unwrap().status(),
            StatusCode::OK
        );
        for (method, path, body) in [
            ("GET", format!("/api/v1/profiles/{a}/catalog"), Value::Null),
            (
                "PUT",
                format!("/api/v1/profiles/{a}/media/{id}/watched"),
                json!({"watched":true}),
            ),
            (
                "PUT",
                format!("/api/v1/profiles/{a}/media/{id}/rating"),
                json!({"rating":5}),
            ),
            (
                "PUT",
                format!("/api/v1/profiles/{a}/preferences"),
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
                &format!("/api/v1/profiles/{b}/media/{id}/rating"),
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
