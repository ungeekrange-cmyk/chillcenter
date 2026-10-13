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
    sync::{Arc, Mutex},
};
use tokio::sync::{mpsc, oneshot};

type Job = Box<dyn FnOnce(&mut Store) + Send>;
#[derive(Clone)]
pub struct DatabaseActor {
    sender: mpsc::Sender<Job>,
}
impl DatabaseActor {
    pub fn open(path: PathBuf) -> Result<Self, String> {
        let (sender, mut receiver) = mpsc::channel::<Job>(16);
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("chill-sqlite".into())
            .spawn(move || {
                let mut store = match Store::open(&path) {
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
        let (tx, rx) = oneshot::channel();
        self.sender
            .try_send(Box::new(move |store| {
                let _ = tx.send(job(store));
            }))
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "BUSY"))?;
        rx.await
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "DATABASE_OFFLINE"))?
            .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "INVALID_OPERATION"))
    }
}

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<ServerConfig>,
    pub library: Arc<Mutex<LibrarySettings>>,
    pub database: DatabaseActor,
    pub certificate: Arc<Mutex<CertificateSchedule>>,
}

#[derive(Debug)]
pub struct ApiError(pub StatusCode, pub &'static str);
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({"code":self.1,"message":"Opération refusée ou temporairement indisponible.","request_id":uuid::Uuid::new_v4().to_string()}))).into_response()
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/server", get(server))
        .route("/api/v1/admin/library", get(library).put(update_library))
        .route("/api/v1/profiles", get(profiles).post(create_profile))
        .route(
            "/api/v1/profiles/{profile}",
            put(rename_profile).delete(delete_profile),
        )
        .route(
            "/api/v1/profiles/{profile}/preferences",
            get(preferences).put(save_preferences),
        )
        .layer(axum::extract::DefaultBodyLimit::max(1024 * 1024))
        .layer(middleware::from_fn_with_state(state.clone(), same_origin))
        .with_state(state)
}

/// No CORS. Native Rust requests have no Origin; any browser-origin request is
/// refused for now (web client is explicitly deferred). Host must name this server.
async fn same_origin(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    if request.headers().contains_key("origin") || request.headers().contains_key("sec-fetch-site")
    {
        return Err(ApiError(StatusCode::FORBIDDEN, "BROWSER_ORIGIN_REFUSED"));
    }
    let host = request
        .headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .or_else(|| request.uri().authority().map(|a| a.as_str()))
        .ok_or(ApiError(StatusCode::BAD_REQUEST, "HOST_REQUIRED"))?;
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
    Ok(next.run(request).await)
}

async fn server(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let certificate = state
        .certificate
        .lock()
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "BUSY"))?
        .clone();
    Ok(Json(
        json!({"server_id":certificate.server_id,"api_version":1,"version":env!("CARGO_PKG_VERSION"),"capabilities":["profiles","configurable_library"],"tls":{"renew_at":certificate.renew_at,"expires_at":certificate.expires_at}}),
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
        settings
            .update(&state.config, body.video_root, body.expected_revision)
            .map_err(|error| {
                if error == "CONFIG_CONFLICT" {
                    ApiError(StatusCode::CONFLICT, "CONFIG_CONFLICT")
                } else {
                    ApiError(StatusCode::BAD_REQUEST, "CONFIG_INVALID_OR_UNWRITABLE")
                }
            })?;
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
    Json(body): Json<Name>,
) -> Result<Json<Profile>, ApiError> {
    Ok(Json(
        state
            .database
            .call(move |store| store.rename_profile(&profile, &body.name))
            .await?,
    ))
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
    state
        .database
        .call(move |store| store.delete_profile(&profile))
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
    async fn http2_authority_without_host_is_accepted() {
        let (_dir, app) = setup();
        let request = Request::builder()
            .version(axum::http::Version::HTTP_2)
            .uri("https://localhost:8443/api/v1/server")
            .body(Body::empty())
            .unwrap();
        assert_eq!(app.oneshot(request).await.unwrap().status(), StatusCode::OK);
    }
}
