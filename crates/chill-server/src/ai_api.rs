use crate::api::{ApiError, AppState};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use chill_domain::ai::{AiApply, AiContextQuery};
use serde_json::json;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/profiles/{profile}/ai/context", get(context))
        .route("/api/v1/profiles/{profile}/ai/apply", post(apply))
}

async fn context(
    State(state): State<AppState>,
    Path(profile): Path<String>,
    Query(query): Query<AiContextQuery>,
) -> Result<Json<chill_domain::ai::AiContext>, ApiError> {
    state
        .database
        .call(move |s| s.ai_context(&profile, query))
        .await
        .map(Json)
}
async fn apply(
    State(state): State<AppState>,
    Path(profile): Path<String>,
    Json(body): Json<AiApply>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Mirror the manual watched route: an explicit unwatch invalidates any
    // in-memory final-credit cache after the database transaction succeeds.
    let reset_final: Vec<String> = body
        .changes
        .iter()
        .filter(|change| change.patch.watched == Some(false))
        .map(|change| change.id.clone())
        .collect();
    let playback = state.playback.clone();
    let apply_profile = profile.clone();
    let out = state
        .database
        .call(move |s| s.ai_apply(&apply_profile, body))
        .await?;
    for media in reset_final {
        playback.reset_final(&profile, &media);
    }
    Ok(Json(json!(out)))
}
