use crate::api::{ApiError, AppState};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};
use chill_domain::reassociation::ReassociationConfirmation;
use serde::Deserialize;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/admin/reassociations", get(proposals).post(confirm))
        .route(
            "/admin/media/{media}/fingerprint",
            get(fingerprint_status).post(request_fingerprint),
        )
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FingerprintRequest {
    complete: bool,
}

async fn request_fingerprint(
    State(state): State<AppState>,
    Path(media): Path<String>,
    Json(body): Json<FingerprintRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if !body.complete || uuid::Uuid::parse_str(&media).is_err() {
        return Err(ApiError(
            StatusCode::UNPROCESSABLE_ENTITY,
            "FINGERPRINT_INVALID",
        ));
    }
    crate::fingerprint::enqueue_full(state, media.clone()).await?;
    Ok(Json(
        serde_json::json!({"media_id":media,"state":"queued","strength":"full"}),
    ))
}

async fn fingerprint_status(
    State(state): State<AppState>,
    Path(media): Path<String>,
) -> Result<Json<chill_domain::fingerprint::FingerprintStatus>, ApiError> {
    if uuid::Uuid::parse_str(&media).is_err() {
        return Err(ApiError(
            StatusCode::UNPROCESSABLE_ENTITY,
            "FINGERPRINT_INVALID",
        ));
    }
    Ok(Json(
        state
            .database
            .call(move |store| store.fingerprint_status(&media))
            .await?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        api::{DatabaseActor, router},
        config::{LibrarySettings, ServerConfig},
    };
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use serde_json::{Value, json};
    use std::sync::{Arc, Mutex};
    use tower::ServiceExt;

    #[tokio::test]
    async fn proof_api_accepts_only_uuid_and_never_exposes_path_or_hash() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let root = base.join("videos");
        std::fs::create_dir(&root).unwrap();
        let path = root.join("secret-name.mp4");
        std::fs::write(&path, b"fixture bytes").unwrap();
        let config = ServerConfig {
            data_dir: base.join("data"),
            video_root: Some(root.clone()),
            ..Default::default()
        };
        std::fs::create_dir(&config.data_dir).unwrap();
        let database = DatabaseActor::open(config.data_dir.join("catalog.sqlite")).unwrap();
        let scan_root = root.clone();
        let scan_path = path.clone();
        let media = database
            .call(move |store| {
                let profile = store.create_profile("A")?;
                store.snapshot(vec![chill_catalog::scanner::parse_file(
                    &scan_root, &scan_path,
                )])?;
                Ok(store.catalog(&profile.id)?[0].id.clone())
            })
            .await
            .unwrap();
        let certificates = crate::tls::ensure(&config, time::OffsetDateTime::now_utc()).unwrap();
        let state = AppState {
            tmdb_jobs: Arc::new(crate::tmdb_api::TmdbJobService::default()),
            playback: Default::default(),
            scan: Arc::new(Mutex::new(crate::catalog_api::ScanStatus::default())),
            database,
            library: Arc::new(Mutex::new(LibrarySettings::load(&config).unwrap())),
            config: Arc::new(config),
            certificate: Arc::new(Mutex::new(certificates.schedule)),
        };
        let app = router(state);
        let invalid = Request::builder()
            .method("POST")
            .uri("/api/v1/admin/media/not-a-uuid/fingerprint")
            .header("host", "localhost:8443")
            .header("content-type", "application/json")
            .body(Body::from(json!({"complete":true}).to_string()))
            .unwrap();
        assert_eq!(
            app.clone().oneshot(invalid).await.unwrap().status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );

        let request = Request::builder()
            .method("POST")
            .uri(format!("/api/v1/admin/media/{media}/fingerprint"))
            .header("host", "localhost:8443")
            .header("content-type", "application/json")
            .body(Body::from(json!({"complete":true}).to_string()))
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body: Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(body.as_object().unwrap().len(), 3);
        assert!(body.get("media_id").is_some());
        assert!(body.to_string().find("secret-name").is_none());
        assert!(body.get("sha256").is_none());

        let mut status = Value::Null;
        for _ in 0..50 {
            let request = Request::builder()
                .uri(format!("/api/v1/admin/media/{media}/fingerprint"))
                .header("host", "localhost:8443")
                .body(Body::empty())
                .unwrap();
            let response = app.clone().oneshot(request).await.unwrap();
            if response.status() == StatusCode::OK {
                status = serde_json::from_slice(
                    &response.into_body().collect().await.unwrap().to_bytes(),
                )
                .unwrap();
                if status["state"] == "complete" {
                    break;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert_eq!(status["state"], "complete");
        assert_eq!(status["strength"], "full");
        assert_eq!(status.as_object().unwrap().len(), 3);
        let serialized = status.to_string();
        assert!(!serialized.contains("secret-name"));
        assert!(!serialized.contains("sha256"));
    }
}
