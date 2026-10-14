//! Explicit TMDB administration actions.  Provider credentials and provider
//! URLs stop here; client DTOs only receive local catalogue artwork endpoints.
use crate::api::{ApiError, AppState};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use chill_domain::catalog_browser::BrowserQuery;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};
#[path = "tmdb_worker.rs"]
mod worker;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/admin/tmdb",
            get(status).put(configure).delete(clear),
        )
        .route("/api/v1/admin/tmdb/jobs", post(start_job))
        .route("/api/v1/admin/tmdb/jobs/{id}", get(job_status))
        .route("/api/v1/admin/tmdb/jobs/{id}/cancel", post(cancel_job))
        .route("/api/v1/admin/tmdb/jobs/{id}/retry", post(retry_job))
        .route(
            "/api/v1/admin/profiles/{profile}/media/{media}/tmdb/candidates",
            get(candidates),
        )
        .route(
            "/api/v1/admin/profiles/{profile}/media/{media}/tmdb",
            post(select),
        )
        .route(
            "/api/v1/admin/profiles/{profile}/media/{media}/tmdb/refresh",
            post(refresh),
        )
        .route(
            "/api/v1/admin/profiles/{profile}/media/{media}/tmdb/poster/restore",
            post(restore_poster),
        )
}

const CREDENTIAL_FILE: &str = "tmdb-token";

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum CredentialType {
    ApiKey,
    Bearer,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StoredCredential {
    credential_type: CredentialType,
    credential: String,
    auto_enrich: bool,
    auto_profile_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct TmdbStatus {
    configured: bool,
    credential_type: Option<CredentialType>,
    source: Option<&'static str>,
    auto_enrich: bool,
}

fn stored_credential(state: &AppState) -> Result<Option<StoredCredential>, ApiError> {
    let path = state.config.data_dir.join(CREDENTIAL_FILE);
    match std::fs::read_to_string(path) {
        Ok(value) if !value.trim().is_empty() => serde_json::from_str(&value)
            .map(Some)
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "TMDB_CREDENTIAL_INVALID")),
        Ok(_) => Err(ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "TMDB_CREDENTIAL_INVALID",
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "TMDB_CREDENTIAL_UNAVAILABLE",
        )),
    }
}
fn configured_token(state: &AppState) -> Result<(String, &'static str), ApiError> {
    resolve_credential(
        stored_credential(state)?,
        std::env::var("CHILL_TMDB_TOKEN").ok(),
    )
}
fn resolve_credential(
    stored: Option<StoredCredential>,
    environment: Option<String>,
) -> Result<(String, &'static str), ApiError> {
    if let Some(stored) = stored {
        let credential = match stored.credential_type {
            CredentialType::ApiKey => stored.credential,
            CredentialType::Bearer => format!("Bearer {}", stored.credential),
        };
        return Ok((credential, "stored"));
    }
    environment
        .filter(|v| !v.trim().is_empty())
        .map(|token| (token, "environment"))
        .ok_or(ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "TMDB_NOT_CONFIGURED",
        ))
}
fn environment_kind(value: &str) -> CredentialType {
    let value = value.trim();
    if value.len() == 32 && value.bytes().all(|b| b.is_ascii_hexdigit()) {
        CredentialType::ApiKey
    } else {
        CredentialType::Bearer
    }
}
fn provider(state: &AppState) -> Result<chill_catalog::metadata::Tmdb, ApiError> {
    chill_catalog::metadata::Tmdb::new(configured_token(state)?.0)
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "TMDB_NOT_CONFIGURED"))
}
fn unavailable(_: String) -> ApiError {
    ApiError(StatusCode::BAD_GATEWAY, "TMDB_UNAVAILABLE")
}

/// One local, redacted enrichment queue.  It owns no credential and only
/// persists user-visible counters/state, so an interrupted process cannot
/// leak a provider secret through job recovery.
#[derive(Clone)]
pub struct TmdbJobService {
    jobs: Arc<Mutex<BTreeMap<String, TmdbJob>>>,
    state_path: Arc<Mutex<Option<PathBuf>>>,
    worker_gate: Arc<tokio::sync::Mutex<()>>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum JobStatus {
    Queued,
    Running,
    PausedPlayback,
    Cancelled,
    Completed,
    Failed,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TmdbJob {
    pub job_id: String,
    status: JobStatus,
    total: usize,
    processed: usize,
    matched: usize,
    ambiguous: usize,
    skipped: usize,
    failed: usize,
    retryable: usize,
    profile_id: String,
    media_ids: Vec<String>,
    #[serde(skip)]
    cancelled: bool,
}
/// Compact polling DTO. Internal profile/media identifiers remain on the
/// server for recovery, rather than being resent with every status refresh.
#[derive(Serialize)]
pub struct TmdbJobView {
    job_id: String,
    status: JobStatus,
    total: usize,
    processed: usize,
    matched: usize,
    ambiguous: usize,
    skipped: usize,
    failed: usize,
    retryable: usize,
}
impl TmdbJob {
    fn view(&self) -> TmdbJobView {
        TmdbJobView {
            job_id: self.job_id.clone(),
            status: self.status.clone(),
            total: self.total,
            processed: self.processed,
            matched: self.matched,
            ambiguous: self.ambiguous,
            skipped: self.skipped,
            failed: self.failed,
            retryable: self.retryable,
        }
    }
}
impl Default for TmdbJobService {
    fn default() -> Self {
        Self {
            jobs: Arc::new(Mutex::new(BTreeMap::new())),
            state_path: Arc::new(Mutex::new(None)),
            worker_gate: Arc::new(tokio::sync::Mutex::new(())),
        }
    }
}
impl TmdbJobService {
    pub fn initialize(data_dir: &std::path::Path) -> Result<Self, String> {
        let service = Self::default();
        let path = data_dir.join("tmdb-jobs.json");
        if let Ok(bytes) = std::fs::read(&path) {
            let mut jobs: BTreeMap<String, TmdbJob> =
                serde_json::from_slice(&bytes).map_err(|_| "État TMDB invalide.")?;
            // A process cannot resume an in-flight HTTP request.  Retain its
            // counters and expose an explicit retryable terminal state.
            for job in jobs.values_mut() {
                if matches!(
                    job.status,
                    JobStatus::Queued | JobStatus::Running | JobStatus::PausedPlayback
                ) {
                    job.status = JobStatus::Failed;
                    job.retryable = job.retryable.saturating_add(1);
                }
            }
            *service.jobs.lock().map_err(|_| "État TMDB indisponible.")? = jobs;
        }
        *service
            .state_path
            .lock()
            .map_err(|_| "État TMDB indisponible.")? = Some(path);
        service.persist()?;
        Ok(service)
    }
    fn persist(&self) -> Result<(), String> {
        let Some(path) = self
            .state_path
            .lock()
            .map_err(|_| "État TMDB indisponible.")?
            .clone()
        else {
            return Ok(());
        };
        let jobs = self
            .jobs
            .lock()
            .map_err(|_| "État TMDB indisponible.")?
            .clone();
        let bytes = serde_json::to_vec(&jobs).map_err(|_| "État TMDB indisponible.")?;
        crate::config::atomic_private_write(&path, &bytes)
    }
    pub fn status(&self, id: &str) -> Option<TmdbJob> {
        self.jobs.lock().ok()?.get(id).cloned()
    }
    pub fn cancel(&self, id: &str) -> Result<TmdbJob, ApiError> {
        let out = {
            let mut jobs = self
                .jobs
                .lock()
                .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "TMDB_JOB_UNAVAILABLE"))?;
            let job = jobs
                .get_mut(id)
                .ok_or(ApiError(StatusCode::NOT_FOUND, "TMDB_JOB_NOT_FOUND"))?;
            if !matches!(job.status, JobStatus::Completed | JobStatus::Failed) {
                job.cancelled = true;
                job.status = JobStatus::Cancelled;
            }
            job.clone()
        };
        self.persist()
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "TMDB_JOB_UNAVAILABLE"))?;
        Ok(out)
    }
    fn cancel_all(&self) {
        if let Ok(mut jobs) = self.jobs.lock() {
            for job in jobs.values_mut() {
                if matches!(
                    job.status,
                    JobStatus::Queued | JobStatus::Running | JobStatus::PausedPlayback
                ) {
                    job.cancelled = true;
                    job.status = JobStatus::Cancelled;
                }
            }
        }
        let _ = self.persist();
    }
    fn create(&self, profile_id: String, mut media_ids: Vec<String>) -> Result<TmdbJob, ApiError> {
        let mut seen = std::collections::BTreeSet::new();
        media_ids.retain(|id| seen.insert(id.clone()));
        let job = TmdbJob {
            job_id: uuid::Uuid::new_v4().to_string(),
            status: JobStatus::Queued,
            total: media_ids.len(),
            processed: 0,
            matched: 0,
            ambiguous: 0,
            skipped: 0,
            failed: 0,
            retryable: 0,
            profile_id,
            media_ids,
            cancelled: false,
        };
        self.jobs
            .lock()
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "TMDB_JOB_UNAVAILABLE"))?
            .insert(job.job_id.clone(), job.clone());
        self.persist()
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "TMDB_JOB_UNAVAILABLE"))?;
        Ok(job)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StartJob {
    profile_id: String,
    media_ids: Option<Vec<String>>,
}
async fn eligible_ids(state: &AppState, profile: &str) -> Result<Vec<String>, ApiError> {
    let mut offset = 0;
    let mut ids = Vec::new();
    loop {
        let profile = profile.to_owned();
        let page = state
            .database
            .call(move |s| {
                s.browse_catalog(
                    &profile,
                    &BrowserQuery {
                        offset,
                        limit: 100,
                        grouped: false,
                        available: Some(true),
                        ..Default::default()
                    },
                )
            })
            .await?;
        ids.extend(
            page.items
                .into_iter()
                .filter(|m| m.tmdb_id.is_none() && !m.poster_custom && !m.series_manual)
                .map(|m| m.id),
        );
        if !page.has_more {
            break;
        }
        offset = offset.saturating_add(100);
    }
    Ok(ids)
}
pub async fn enqueue_auto(state: AppState) {
    let Ok(Some(config)) = stored_credential(&state) else {
        return;
    };
    if !config.auto_enrich {
        return;
    }
    let Some(profile) = config.auto_profile_id else {
        return;
    };
    let Ok(ids) = eligible_ids(&state, &profile).await else {
        return;
    };
    let Ok(job) = state.tmdb_jobs.create(profile, ids) else {
        return;
    };
    let service = state.tmdb_jobs.clone();
    tokio::spawn(async move {
        run_job(state, service, job.job_id).await;
    });
}
fn exact_candidate(
    media: &chill_catalog::model::Media,
    candidates: Vec<chill_catalog::metadata::Candidate>,
) -> Option<chill_catalog::metadata::Candidate> {
    let label = media.series.as_deref().unwrap_or(&media.title);
    let norm = |v: &str| {
        v.chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect::<String>()
    };
    let kind = if media.category == "serie" {
        "tv"
    } else {
        "movie"
    };
    let matches = candidates
        .into_iter()
        .filter(|c| {
            c.kind == kind
                && norm(&c.title) == norm(label)
                && media.year.is_none_or(|y| c.year == Some(y))
        })
        .collect::<Vec<_>>();
    (matches.len() == 1).then(|| matches.into_iter().next().unwrap())
}
async fn run_job(state: AppState, service: Arc<TmdbJobService>, id: String) {
    let playback = state.playback.clone();
    worker::run(
        state,
        service,
        id,
        None,
        Arc::new(move || playback.fingerprint_should_pause()),
        worker::Policy::default(),
    )
    .await;
}
fn bump(service: &TmdbJobService, id: &str, update: impl FnOnce(&mut TmdbJob)) {
    if let Ok(mut jobs) = service.jobs.lock()
        && let Some(job) = jobs.get_mut(id)
        && !job.cancelled
        && !matches!(job.status, JobStatus::Cancelled)
    {
        update(job)
    }
    let _ = service.persist();
}

#[cfg(test)]
mod job_tests {
    use super::*;
    fn media(title: &str, year: Option<i32>) -> chill_catalog::model::Media {
        serde_json::from_value(serde_json::json!({"id":"x","path":"/x","title":title,"category":"film","year":year,"series":null,"season":null,"episode":null,"genres":[],"overview":"","poster":null,"saga":null,"watched":false,"available":true,"added_at":0,"tmdb_id":null})).unwrap()
    }
    fn candidate(title: &str, year: Option<i32>) -> chill_catalog::metadata::Candidate {
        chill_catalog::metadata::Candidate {
            id: 1,
            title: title.into(),
            year,
            overview: String::new(),
            poster: None,
            kind: "movie".into(),
        }
    }
    #[test]
    fn exact_match_refuses_ambiguity_and_year_mismatch() {
        assert!(
            exact_candidate(
                &media("Alien!", Some(1979)),
                vec![candidate("Alien", Some(1979))]
            )
            .is_some()
        );
        assert!(
            exact_candidate(
                &media("Alien", Some(1979)),
                vec![
                    candidate("Alien", Some(1979)),
                    candidate("Alien", Some(1979))
                ]
            )
            .is_none()
        );
        assert!(
            exact_candidate(
                &media("Alien", Some(1979)),
                vec![candidate("Alien", Some(1986))]
            )
            .is_none()
        );
    }
    #[test]
    fn recovered_active_job_is_retryable_without_media_or_secret() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tmdb-jobs.json");
        std::fs::write(&path, br#"{"j":{"job_id":"j","status":"running","total":1,"processed":0,"matched":0,"ambiguous":0,"skipped":0,"failed":0,"retryable":0,"profile_id":"p","media_ids":["m"]}}"#).unwrap();
        let service = TmdbJobService::initialize(dir.path()).unwrap();
        let job = service.status("j").unwrap();
        assert!(matches!(job.status, JobStatus::Failed));
        assert_eq!(job.retryable, 1);
        assert!(
            !std::fs::read_to_string(path)
                .unwrap()
                .contains("credential")
        );
    }
    #[test]
    fn polling_dto_stays_compact_while_recovery_retains_full_worklist() {
        let service = TmdbJobService::default();
        let ids = (0..2496)
            .map(|_| uuid::Uuid::new_v4().to_string())
            .collect::<Vec<_>>();
        let job = service.create("synthetic-profile".into(), ids).unwrap();
        let dto = serde_json::to_string(&job.view()).unwrap();
        assert!(dto.len() < 500);
        assert!(!dto.contains("media_ids"));
        assert!(!dto.contains("profile_id"));
        assert!(!dto.contains("synthetic-profile"));
        assert_eq!(
            serde_json::to_value(&job).unwrap()["media_ids"]
                .as_array()
                .unwrap()
                .len(),
            2496
        );
        assert_eq!(serde_json::to_value(job.view()).unwrap()["retryable"], 0);
    }
}
async fn start_job(
    State(state): State<AppState>,
    Json(body): Json<StartJob>,
) -> Result<(StatusCode, Json<TmdbJobView>), ApiError> {
    if body.media_ids.as_ref().is_some_and(|ids| ids.len() > 3000) {
        return Err(ApiError(
            StatusCode::PAYLOAD_TOO_LARGE,
            "TMDB_JOB_TOO_LARGE",
        ));
    }
    state
        .database
        .call({
            let profile = body.profile_id.clone();
            move |s| s.preferences(&profile).map(|_| ())
        })
        .await?;
    let profile = body.profile_id;
    let ids = if let Some(ids) = body.media_ids {
        ids
    } else {
        eligible_ids(&state, &profile).await?
    };
    let job = state.tmdb_jobs.create(profile, ids)?;
    let worker_state = state.clone();
    let worker = state.tmdb_jobs.clone();
    let id = job.job_id.clone();
    tokio::spawn(async move {
        run_job(worker_state, worker, id).await;
    });
    Ok((StatusCode::ACCEPTED, Json(job.view())))
}
async fn job_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<TmdbJobView>, ApiError> {
    state
        .tmdb_jobs
        .status(&id)
        .map(|job| Json(job.view()))
        .ok_or(ApiError(StatusCode::NOT_FOUND, "TMDB_JOB_NOT_FOUND"))
}
async fn cancel_job(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<TmdbJobView>, ApiError> {
    state.tmdb_jobs.cancel(&id).map(|job| Json(job.view()))
}
async fn retry_job(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<(StatusCode, Json<TmdbJobView>), ApiError> {
    let prior = state
        .tmdb_jobs
        .status(&id)
        .ok_or(ApiError(StatusCode::NOT_FOUND, "TMDB_JOB_NOT_FOUND"))?;
    if !matches!(prior.status, JobStatus::Failed | JobStatus::Cancelled) {
        return Err(ApiError(StatusCode::CONFLICT, "TMDB_JOB_NOT_RETRYABLE"));
    }
    let job = state.tmdb_jobs.create(prior.profile_id, prior.media_ids)?;
    let worker_state = state.clone();
    let worker = state.tmdb_jobs.clone();
    let job_id = job.job_id.clone();
    tokio::spawn(async move {
        run_job(worker_state, worker, job_id).await;
    });
    Ok((StatusCode::ACCEPTED, Json(job.view())))
}

async fn status(State(state): State<AppState>) -> Result<Json<TmdbStatus>, ApiError> {
    let status = match stored_credential(&state)? {
        Some(stored) => TmdbStatus {
            configured: true,
            credential_type: Some(stored.credential_type),
            source: Some("stored"),
            auto_enrich: stored.auto_enrich,
        },
        None if std::env::var("CHILL_TMDB_TOKEN")
            .ok()
            .is_some_and(|v| !v.trim().is_empty()) =>
        {
            TmdbStatus {
                configured: true,
                credential_type: std::env::var("CHILL_TMDB_TOKEN")
                    .ok()
                    .map(|value| environment_kind(&value)),
                source: Some("environment"),
                auto_enrich: false,
            }
        }
        None => TmdbStatus {
            configured: false,
            credential_type: None,
            source: None,
            auto_enrich: false,
        },
    };
    Ok(Json(status))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigureTmdb {
    credential_type: CredentialType,
    credential: String,
    auto_enrich: bool,
    profile_id: Option<String>,
}
async fn configure(
    State(state): State<AppState>,
    Json(body): Json<ConfigureTmdb>,
) -> Result<Json<TmdbStatus>, ApiError> {
    let input = body.credential.trim();
    let bearer = input.strip_prefix("Bearer ").unwrap_or(input);
    let credential = match body.credential_type {
        CredentialType::ApiKey
            if input.len() == 32 && input.bytes().all(|b| b.is_ascii_hexdigit()) =>
        {
            input.to_owned()
        }
        CredentialType::Bearer
            if (20..=2048).contains(&bearer.len())
                && bearer
                    .bytes()
                    .all(|b| b.is_ascii_graphic() && !b.is_ascii_whitespace()) =>
        {
            bearer.to_owned()
        }
        _ => return Err(ApiError(StatusCode::BAD_REQUEST, "TMDB_CREDENTIAL_INVALID")),
    };
    if body.auto_enrich && body.profile_id.as_deref().is_none_or(str::is_empty) {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "TMDB_AUTO_PROFILE_REQUIRED",
        ));
    }
    if let Some(profile) = body.profile_id.clone() {
        state
            .database
            .call(move |s| s.preferences(&profile).map(|_| ()))
            .await?;
    }
    let stored = StoredCredential {
        credential_type: body.credential_type,
        credential,
        auto_enrich: body.auto_enrich,
        auto_profile_id: body.profile_id,
    };
    let bytes = serde_json::to_vec(&stored).map_err(|_| {
        ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "TMDB_CREDENTIAL_UNAVAILABLE",
        )
    })?;
    if chill_catalog::metadata::Tmdb::new(match stored.credential_type {
        CredentialType::ApiKey => stored.credential.clone(),
        CredentialType::Bearer => format!("Bearer {}", stored.credential),
    })
    .is_err()
    {
        return Err(ApiError(StatusCode::BAD_REQUEST, "TMDB_CREDENTIAL_INVALID"));
    }
    crate::config::atomic_private_write(&state.config.data_dir.join(CREDENTIAL_FILE), &bytes)
        .map_err(|_| {
            ApiError(
                StatusCode::SERVICE_UNAVAILABLE,
                "TMDB_CREDENTIAL_UNAVAILABLE",
            )
        })?;
    state.tmdb_jobs.cancel_all();
    Ok(Json(TmdbStatus {
        configured: true,
        credential_type: Some(stored.credential_type),
        source: Some("stored"),
        auto_enrich: stored.auto_enrich,
    }))
}
async fn clear(State(state): State<AppState>) -> Result<Json<TmdbStatus>, ApiError> {
    state.tmdb_jobs.cancel_all();
    let path = state.config.data_dir.join(CREDENTIAL_FILE);
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => {
            return Err(ApiError(
                StatusCode::SERVICE_UNAVAILABLE,
                "TMDB_CREDENTIAL_UNAVAILABLE",
            ));
        }
    }
    // Durably remove the directory entry just as atomic_private_write durably
    // installs it; a successful response must not precede this fsync.
    std::fs::File::open(&state.config.data_dir)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| {
            ApiError(
                StatusCode::SERVICE_UNAVAILABLE,
                "TMDB_CREDENTIAL_UNAVAILABLE",
            )
        })?;
    let environment = std::env::var("CHILL_TMDB_TOKEN")
        .ok()
        .is_some_and(|v| !v.trim().is_empty());
    Ok(Json(TmdbStatus {
        configured: environment,
        credential_type: environment
            .then(|| environment_kind(&std::env::var("CHILL_TMDB_TOKEN").unwrap_or_default())),
        source: environment.then_some("environment"),
        auto_enrich: false,
    }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateQuery {
    query: Option<String>,
}
#[derive(Serialize)]
struct CandidateDto {
    id: i64,
    title: String,
    year: Option<i32>,
    overview: String,
    kind: String,
    poster_available: bool,
}
async fn candidates(
    State(state): State<AppState>,
    Path((profile, media)): Path<(String, String)>,
    Query(query): Query<CandidateQuery>,
) -> Result<Json<Value>, ApiError> {
    let item = state
        .database
        .call(move |s| s.media(&profile, &media))
        .await?;
    let results = provider(&state)?
        .candidates(&item, query.query.as_deref().unwrap_or(""))
        .await
        .map_err(unavailable)?;
    // Never hand a third-party image URL to a webview.  Selection obtains a
    // local validated cache copy before the DTO exposes artwork.
    let candidates = results
        .into_iter()
        .map(|c| CandidateDto {
            id: c.id,
            title: c.title,
            year: c.year,
            overview: c.overview,
            kind: c.kind,
            poster_available: c.poster.is_some(),
        })
        .collect::<Vec<_>>();
    Ok(Json(json!({"candidates": candidates})))
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Scope {
    Media,
    Series,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Select {
    expected_revision: u64,
    tmdb_id: i64,
    kind: String,
    scope: Scope,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Refresh {
    expected_revision: u64,
    scope: Scope,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Restore {
    expected_revision: u64,
}

// Explicit provider/CAS inputs shared by selection and refresh handlers.
#[allow(clippy::too_many_arguments)]
async fn apply(
    state: AppState,
    profile: String,
    media: String,
    expected_revision: u64,
    id: i64,
    kind: String,
    scope: Scope,
    explicit: bool,
) -> Result<Json<Value>, ApiError> {
    let cache = state.config.data_dir.join("posters");
    if scope == Scope::Series {
        let snapshot = state
            .database
            .call({
                let profile = profile.clone();
                let media = media.clone();
                move |s| s.provider_series_snapshot(&profile, &media, expected_revision)
            })
            .await?;
        if kind != "tv" {
            return Err(ApiError(
                StatusCode::UNPROCESSABLE_ENTITY,
                "TMDB_SCOPE_INVALID",
            ));
        }
        let provider = provider(&state)?
            .series_metadata(
                snapshot.members.first().ok_or(ApiError(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "TMDB_SCOPE_INVALID",
                ))?,
                id,
                &cache,
            )
            .await
            .map_err(unavailable)?;
        let owned = provider.poster.clone();
        let updated = match state
            .database
            .call(move |s| {
                s.provider_apply_series(&profile, &media, &snapshot, &provider, explicit)
            })
            .await
        {
            Ok(updated) => updated,
            Err(error) => {
                if let Some(path) = owned {
                    let _ = std::fs::remove_file(path);
                }
                return Err(error);
            }
        };
        Ok(Json(
            json!({"media": updated, "revision": expected_revision + 1}),
        ))
    } else {
        let snapshot = state
            .database
            .call({
                let profile = profile.clone();
                let media = media.clone();
                move |s| s.provider_snapshot(&profile, &media, expected_revision)
            })
            .await?;
        let downloaded = provider(&state)?
            .enrich(snapshot.media.clone(), id, &kind, &cache)
            .await
            .map_err(unavailable)?;
        let owned = downloaded
            .poster
            .clone()
            .filter(|path| snapshot.media.poster.as_ref() != Some(path));
        let (media, revision) = match state
            .database
            .call(move |s| {
                s.provider_apply_media(&profile, &media, &snapshot, downloaded, explicit)
            })
            .await
        {
            Ok(value) => value,
            Err(error) => {
                if let Some(path) = owned {
                    let _ = std::fs::remove_file(path);
                }
                return Err(error);
            }
        };
        Ok(Json(json!({"media": media, "revision": revision})))
    }
}

async fn select(
    State(state): State<AppState>,
    Path((profile, media)): Path<(String, String)>,
    Json(body): Json<Select>,
) -> Result<Json<Value>, ApiError> {
    apply(
        state,
        profile,
        media,
        body.expected_revision,
        body.tmdb_id,
        body.kind,
        body.scope,
        true,
    )
    .await
}
async fn refresh(
    State(state): State<AppState>,
    Path((profile, media)): Path<(String, String)>,
    Json(body): Json<Refresh>,
) -> Result<Json<Value>, ApiError> {
    let source = state
        .database
        .call({
            let profile = profile.clone();
            let media = media.clone();
            move |s| s.provider_snapshot(&profile, &media, body.expected_revision)
        })
        .await?;
    let id = source.media.tmdb_id.ok_or(ApiError(
        StatusCode::UNPROCESSABLE_ENTITY,
        "TMDB_ASSOCIATION_MISSING",
    ))?;
    let kind = if source.media.category == "serie" {
        "tv"
    } else {
        "movie"
    }
    .to_owned();
    apply(
        state,
        profile,
        media,
        body.expected_revision,
        id,
        kind,
        body.scope,
        false,
    )
    .await
}
async fn restore_poster(
    State(state): State<AppState>,
    Path((profile, media)): Path<(String, String)>,
    Json(body): Json<Restore>,
) -> Result<Json<Value>, ApiError> {
    let snapshot = state
        .database
        .call({
            let profile = profile.clone();
            let media = media.clone();
            move |s| s.provider_snapshot(&profile, &media, body.expected_revision)
        })
        .await?;
    let id = snapshot.media.tmdb_id.ok_or(ApiError(
        StatusCode::UNPROCESSABLE_ENTITY,
        "TMDB_ASSOCIATION_MISSING",
    ))?;
    let kind = if snapshot.media.category == "serie" {
        "tv"
    } else {
        "movie"
    };
    let poster = provider(&state)?
        .restore_poster(kind, id, &state.config.data_dir.join("posters"))
        .await
        .map_err(unavailable)?;
    let owned = poster.clone();
    match state
        .database
        .call(move |s| s.provider_restore_poster(&profile, &media, &snapshot, poster))
        .await
    {
        Ok((media, revision)) => Ok(Json(json!({"media":media,"revision":revision}))),
        Err(error) => {
            let _ = std::fs::remove_file(owned);
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_failure_is_a_redacted_gateway_error() {
        let error = unavailable("fake provider failure with token-like text".into());
        assert_eq!(error.0, StatusCode::BAD_GATEWAY);
        assert_eq!(error.1, "TMDB_UNAVAILABLE");
    }

    #[test]
    fn candidate_dto_never_serializes_a_provider_image_url() {
        let candidate = CandidateDto {
            id: 1,
            title: "Film".into(),
            year: Some(2026),
            overview: "Résumé".into(),
            kind: "movie".into(),
            poster_available: true,
        };
        let rendered = serde_json::to_string(&candidate).unwrap();
        assert!(rendered.contains("poster_available"));
        assert!(!rendered.contains("http"));
        assert!(!rendered.contains("poster_path"));
    }
}
