use crate::api::{ApiError, AppState};
use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    routing::get,
};
use chill_domain::reassociation::ReassociationConfirmation;
use serde::Deserialize;

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/v1/admin/reassociations", get(proposals).post(confirm))
}
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Page {
    offset: usize,
    limit: usize,
}
async fn proposals(
    State(state): State<AppState>,
    Query(page): Query<Page>,
) -> Result<Json<chill_domain::reassociation::ReassociationPage>, ApiError> {
    let limit = if page.limit == 0 { 48 } else { page.limit };
    Ok(Json(
        state
            .database
            .call(move |s| s.reassociation_proposals(page.offset, limit))
            .await?,
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Confirm {
    confirmed: bool,
    source_id: String,
    target_id: String,
    expected_source_revision: u64,
    expected_target_revision: u64,
    expected_epoch: u64,
}
async fn confirm(
    State(state): State<AppState>,
    Json(body): Json<Confirm>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if !body.confirmed {
        return Err(ApiError(
            StatusCode::UNPROCESSABLE_ENTITY,
            "CONFIRMATION_REQUIRED",
        ));
    }
    state
        .database
        .call(move |s| {
            s.apply_reassociation(&ReassociationConfirmation {
                source_id: body.source_id,
                target_id: body.target_id,
                expected_source_revision: body.expected_source_revision,
                expected_target_revision: body.expected_target_revision,
                expected_epoch: body.expected_epoch,
            })
        })
        .await?;
    Ok(Json(serde_json::json!({"reassociated":true})))
}
