//! Bounded non-root update intents; the separate signed helper owns installation.
use crate::api::{ApiError, AppState};
use axum::{
    Json, Router,
    extract::State,
    http::{Method, StatusCode},
    routing::{get, post},
};
use chill_updates::{Component, IntentKind, PolicyMode, StateStore, UpdateError};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};
#[path = "update_checks.rs"]
mod checks;
const MAINTENANCE: &str = "/run/chillcenter-update/maintenance";
static MUTATIONS: AtomicUsize = AtomicUsize::new(0);
pub struct MutationGuard;
impl Drop for MutationGuard {
    fn drop(&mut self) {
        MUTATIONS.fetch_sub(1, Ordering::AcqRel);
    }
}
pub fn begin_mutation(method: &Method) -> Option<MutationGuard> {
    if matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS) {
        return None;
    }
    MUTATIONS.fetch_add(1, Ordering::AcqRel);
    Some(MutationGuard)
}
/// Reserve before inspecting maintenance, just like HTTP admission. A root
/// helper cannot observe an idle proof between this check and a TLS write.
pub fn try_background_write() -> Option<MutationGuard> {
    MUTATIONS.fetch_add(1, Ordering::AcqRel);
    let guard = MutationGuard;
    if maintenance_generation().is_some() {
        None
    } else {
        Some(guard)
    }
}
pub fn maintenance_generation() -> Option<String> {
    use std::os::unix::fs::MetadataExt;
    let metadata = std::fs::symlink_metadata(MAINTENANCE).ok()?;
    if !metadata.is_file() || metadata.uid() != 0 || metadata.mode() & 0o022 != 0 {
        return None;
    }
    Some(
        metadata
            .modified()
            .ok()?
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_nanos()
            .to_string(),
    )
}
pub fn admitted_during_maintenance(method: &Method, path: &str) -> bool {
    matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
        || (*method == Method::POST
            && path.contains("/playback/")
            && ["/heartbeat", "/progress", "/release"]
                .iter()
                .any(|end| path.ends_with(end)))
}
fn error(value: UpdateError) -> ApiError {
    match value {
        UpdateError::Busy(_) => ApiError(StatusCode::CONFLICT, "UPDATE_STATE_CHANGED"),
        UpdateError::Invalid(_) => {
            ApiError(StatusCode::UNPROCESSABLE_ENTITY, "UPDATE_REQUEST_INVALID")
        }
        _ => ApiError(StatusCode::SERVICE_UNAVAILABLE, "UPDATE_STATE_UNAVAILABLE"),
    }
}
fn root(state: &AppState) -> PathBuf {
    state.config.data_dir.join("updates")
}
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/admin/updates", get(status))
        .route("/admin/updates/policy", post(policy).put(policy))
        .route("/admin/updates/check", post(check))
        .route("/admin/updates/install", post(install))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    component: Component,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyRequest {
    component: Component,
    mode: PolicyMode,
    hour: u8,
    minute: u8,
    expected_revision: u64,
}
fn server_component(component: Component) -> Result<Component, ApiError> {
    if component == Component::Mac {
        Err(ApiError(
            StatusCode::UNPROCESSABLE_ENTITY,
            "UPDATE_COMPONENT_INVALID",
        ))
    } else {
        Ok(component)
    }
}
async fn policy(
    State(state): State<AppState>,
    Json(request): Json<PolicyRequest>,
) -> Result<Json<Value>, ApiError> {
    let component = server_component(request.component)?;
    let path = root(&state);
    let policy = tokio::task::spawn_blocking(move || {
        StateStore::open(path)?.compare_and_set_policy(
            component,
            request.expected_revision,
            request.mode,
            request.hour,
            request.minute,
        )
    })
    .await
    .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "UPDATE_STATE_UNAVAILABLE"))?
    .map_err(error)?;
    Ok(Json(json!({"component":component,"policy":policy})))
}
async fn check(
    State(state): State<AppState>,
    Json(request): Json<Request>,
) -> Result<Json<Value>, ApiError> {
    let component = server_component(request.component)?;
    if !Path::new("/etc/chillcenter/updates.json").is_file() {
        let path = root(&state).join("checks");
        return tokio::task::spawn_blocking(move || checks::admit(path, component))
            .await
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "UPDATE_STATE_UNAVAILABLE"))?
            .map(Json)
            .map_err(error);
    }
    intent(state, request.component, IntentKind::Check).await
}
async fn install(
    State(state): State<AppState>,
    Json(request): Json<Request>,
) -> Result<Json<Value>, ApiError> {
    intent(state, request.component, IntentKind::Install).await
}
async fn intent(
    state: AppState,
    component: Component,
    kind: IntentKind,
) -> Result<Json<Value>, ApiError> {
    let component = server_component(component)?;
    if !Path::new("/etc/chillcenter/updates.json").is_file() {
        return Err(ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "UPDATE_HELPER_NOT_INSTALLED",
        ));
    }
    let path = root(&state);
    let operation = tokio::task::spawn_blocking(move || {
        StateStore::open(path)?.admit_intent(component, kind, time::OffsetDateTime::now_utc())
    })
    .await
    .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "UPDATE_STATE_UNAVAILABLE"))?
    .map_err(error)?;
    Ok(Json(json!({"operation":operation})))
}
pub fn busy(state: &AppState) -> bool {
    MUTATIONS.load(Ordering::Acquire) != 0
        || checks::busy()
        || state.playback.fingerprint_should_pause()
        || state.scan.lock().map_or(true, |scan| scan.running)
        || state.tmdb_jobs.update_busy()
        || crate::fingerprint::update_busy()
        || crate::archive_api::update_busy(state)
        || crate::export_jobs::update_busy(state)
}
fn bounded_json(path: &Path) -> Option<Value> {
    let metadata = std::fs::symlink_metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > 1024 * 1024 {
        return None;
    }
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}
pub fn installation_root() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| {
            let parent = path.parent()?;
            if parent.parent()?.file_name()? == "releases" {
                Some(parent.parent()?.parent()?.parent()?.to_owned())
            } else {
                Some(parent.to_owned())
            }
        })
        .unwrap_or_else(|| PathBuf::from("/nonexistent/chillcenter"))
}
pub fn installed_versions() -> Value {
    let base = installation_root();
    let web = bounded_json(&base.join("web/current/build.json"))
        .or_else(|| bounded_json(&base.join("web/build.json")));
    let themes = bounded_json(&base.join("themes/current/manifest.json"))
        .or_else(|| bounded_json(&base.join("web/current/themes/manifest.json")))
        .or_else(|| bounded_json(&base.join("web/themes/manifest.json")));
    json!({"server":{"version":env!("CARGO_PKG_VERSION")},"web":web,"themes":themes,"api":crate::api_version::VERSION})
}
async fn status(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let path = root(&state);
    let work_busy = busy(&state);
    let generation = maintenance_generation();
    let value = tokio::task::spawn_blocking(move || {
        let state = StateStore::open(&path)?.read()?;
        let offset = local_offset();
        let helper_available = Path::new("/etc/chillcenter/updates.json").is_file();
        let (operations, available) = if helper_available {
            (state.operations, bounded_json(&path.join("available.json")))
        } else {
            checks::snapshot(&path.join("checks"))?
        };
        Ok::<_, UpdateError>(json!({"installed":installed_versions(),"policies":state.policies,"operations":operations,"components":state.components,
            "available":available,"helper_available":helper_available,"check_available":true,
            "busy":work_busy,"quiescing":generation.is_some(),"active_mutations":MUTATIONS.load(Ordering::Acquire),"timezone":offset.to_string(),"server_time":time::OffsetDateTime::now_utc().unix_timestamp()}))
    }).await.map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "UPDATE_STATE_UNAVAILABLE"))?.map_err(error)?;
    Ok(Json(value))
}
fn local_offset() -> time::UtcOffset {
    let now = unsafe { libc::time(std::ptr::null_mut()) };
    let mut out = std::mem::MaybeUninit::<libc::tm>::uninit();
    if unsafe { libc::localtime_r(&now, out.as_mut_ptr()) }.is_null() {
        return time::UtcOffset::UTC;
    }
    time::UtcOffset::from_whole_seconds(unsafe { out.assume_init() }.tm_gmtoff as i32)
        .unwrap_or(time::UtcOffset::UTC)
}
/// A fresh generation-bound idle proof, sampled independently from UI/database
/// locks. The helper creates the root-owned flag before waiting for this proof.
pub fn spawn_status_pulse(state: AppState) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        loop {
            interval.tick().await;
            let Some(generation) = maintenance_generation() else {
                continue;
            };
            let state = state.clone();
            let _ = tokio::task::spawn_blocking(move || {
                let path = root(&state); std::fs::create_dir_all(&path)?;
                let bytes = serde_json::to_vec(&json!({"quiescing":true,"busy":busy(&state),"active_mutations":MUTATIONS.load(Ordering::Acquire),"updated_unix":time::OffsetDateTime::now_utc().unix_timestamp(),"maintenance_generation_ns":generation}))?;
                use std::io::Write; use std::os::unix::fs::OpenOptionsExt;
                let temporary = path.join(format!(".quiescence-{}",uuid::Uuid::new_v4()));
                let mut file = std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&temporary)?;
                file.write_all(&bytes)?; file.sync_all()?; std::fs::rename(temporary,path.join("quiescence.json"))?;
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
            }).await;
        }
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maintenance_allows_existing_playback_not_new_work() {
        assert!(admitted_during_maintenance(
            &Method::GET,
            "/api/2026-10-14/server"
        ));
        for op in ["heartbeat", "progress", "release"] {
            assert!(admitted_during_maintenance(
                &Method::POST,
                &format!("/api/2026-10-14/profiles/p/playback/{op}")
            ));
        }
        for path in [
            "/api/2026-10-14/profiles/p/playback/acquire",
            "/api/2026-10-14/admin/scan",
            "/api/2026-10-14/admin/updates/install",
        ] {
            assert!(!admitted_during_maintenance(&Method::POST, path));
        }
    }
    #[test]
    fn actual_versions_and_closed_request_fields() {
        assert_eq!(
            installed_versions()["server"]["version"],
            env!("CARGO_PKG_VERSION")
        );
        assert!(
            serde_json::from_str::<Request>(
                r#"{"component":"server","url":"https://bad.invalid/"}"#
            )
            .is_err()
        );
        assert!(serde_json::from_str::<PolicyRequest>(r#"{"component":"server","mode":"automatic","hour":2,"minute":0,"expected_revision":1,"force":true}"#).is_err());
        assert!(server_component(Component::Mac).is_err());
    }
}
