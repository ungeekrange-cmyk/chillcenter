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
    let out = state
        .database
        .call(move |s| s.ai_apply(&profile, body))
        .await?;
    Ok(Json(json!(out)))
}
