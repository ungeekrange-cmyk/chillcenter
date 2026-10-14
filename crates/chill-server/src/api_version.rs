//! Canonical date-versioned contract and safe JSON errors for framework failures.
use axum::{
    Json,
    body::Body,
    http::{HeaderValue, Method, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::json;
pub const VERSION: &str = "2026-10-14";
pub const PREFIX: &str = "/api/2026-10-14";
pub async fn openapi() -> impl IntoResponse {
    // Serve the immutable validated document directly; no parse/serialize work
    // or cloned schema tree per request on the Raspberry Pi.
    (
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        include_str!("../../../docs/api/openapi-2026-10-14.json"),
    )
}
pub async fn normalize_response(request: Request<Body>, next: Next) -> Response {
    let is_head = request.method() == Method::HEAD;
    let mut response = next.run(request).await;
    let status = response.status();
    if (status.is_client_error() || status.is_server_error())
        && !response
            .headers()
            .get("content-type")
            .is_some_and(|value| value.as_bytes().starts_with(b"application/json"))
    {
        let code = match status {
            StatusCode::NOT_FOUND => "ROUTE_NOT_FOUND",
            StatusCode::METHOD_NOT_ALLOWED => "METHOD_NOT_ALLOWED",
            StatusCode::PAYLOAD_TOO_LARGE => "PAYLOAD_TOO_LARGE",
            StatusCode::UNSUPPORTED_MEDIA_TYPE => "CONTENT_TYPE_INVALID",
            StatusCode::UNPROCESSABLE_ENTITY => "VALIDATION_FAILED",
            _ => "INVALID_REQUEST",
        };
        let headers = response.headers().clone();
        response=(status,Json(json!({"code":code,"message":"Requête refusée. Consultez le contrat de l’API.","request_id":uuid::Uuid::new_v4().to_string()}))).into_response();
        // Keep protocol metadata when replacing Axum's textual rejection body.
        // Its old Content-Length/Content-Type describe the discarded bytes.
        for (name, value) in &headers {
            if name != axum::http::header::CONTENT_TYPE
                && name != axum::http::header::CONTENT_LENGTH
            {
                response.headers_mut().append(name.clone(), value.clone());
            }
        }
    }
    if is_head {
        *response.body_mut() = Body::empty();
    }
    response
        .headers_mut()
        .insert("x-chill-api-version", HeaderValue::from_static(VERSION));
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, middleware, routing::get};
    use http_body_util::BodyExt;
    use serde_json::Value;
    use tower::ServiceExt;
    #[tokio::test]
    async fn framework_errors_are_json_and_preserve_http_metadata() {
        let app = Router::new()
            .route(
                "/sample",
                get(|| async {
                    (
                        StatusCode::RANGE_NOT_SATISFIABLE,
                        [("content-range", "bytes */42"), ("retry-after", "5")],
                        "secret internal detail",
                    )
                }),
            )
            .layer(middleware::from_fn(normalize_response));
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/sample")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::RANGE_NOT_SATISFIABLE);
        assert_eq!(response.headers()["content-range"], "bytes */42");
        assert_eq!(response.headers()["retry-after"], "5");
        assert_eq!(response.headers()["x-chill-api-version"], VERSION);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let value: Value = serde_json::from_slice(&body).unwrap();
        assert!(uuid::Uuid::parse_str(value["request_id"].as_str().unwrap()).is_ok());
        assert!(!String::from_utf8_lossy(&body).contains("secret"));
        let method = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/sample")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(method.status(), StatusCode::METHOD_NOT_ALLOWED);
        assert!(method.headers().contains_key("allow"));
        let head = app
            .oneshot(
                Request::builder()
                    .method("HEAD")
                    .uri("/missing")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(head.status(), StatusCode::NOT_FOUND);
        assert!(
            head.into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .is_empty()
        );
    }
}
