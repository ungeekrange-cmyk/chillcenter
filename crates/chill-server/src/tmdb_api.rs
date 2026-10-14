//! Explicit TMDB administration actions.  Provider credentials and provider
//! URLs stop here; client DTOs only receive local catalogue artwork endpoints.
use crate::api::{ApiError, AppState};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/admin/tmdb", get(status))
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

fn token() -> Result<String, ApiError> {
    std::env::var("CHILL_TMDB_TOKEN")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .ok_or(ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "TMDB_NOT_CONFIGURED",
        ))
}
fn provider() -> Result<chill_catalog::metadata::Tmdb, ApiError> {
    chill_catalog::metadata::Tmdb::new(token()?)
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "TMDB_NOT_CONFIGURED"))
}
fn unavailable(_: String) -> ApiError {
    ApiError(StatusCode::BAD_GATEWAY, "TMDB_UNAVAILABLE")
}

async fn status() -> Json<Value> {
    Json(json!({"configured": token().is_ok()}))
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
    let results = provider()?
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
        let provider = provider()?
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
        let downloaded = provider()?
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
    let poster = provider()?
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
