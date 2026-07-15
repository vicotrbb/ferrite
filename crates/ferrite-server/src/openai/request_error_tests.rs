use super::routes::{router, MAX_OPENAI_REQUEST_BODY_BYTES};
use crate::state::ServerState;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn completions_endpoint_returns_openai_error_for_malformed_json(
) -> Result<(), Box<dyn std::error::Error>> {
    let app = router(ServerState::new("fixture-model".to_owned()));
    let request = Request::builder()
        .method("POST")
        .uri("/v1/completions")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"model":"fixture-model","prompt":"hello""#))?;
    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_json(response.into_body()).await?;
    assert_eq!(body["error"]["type"], "invalid_request_error");
    Ok(())
}

#[tokio::test]
async fn completions_endpoint_returns_openai_error_for_missing_json_content_type(
) -> Result<(), Box<dyn std::error::Error>> {
    let app = router(ServerState::new("fixture-model".to_owned()));
    let request = Request::builder()
        .method("POST")
        .uri("/v1/completions")
        .body(Body::from(r#"{"model":"fixture-model","prompt":"hello"}"#))?;
    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_json(response.into_body()).await?;
    assert_eq!(body["error"]["type"], "invalid_request_error");
    Ok(())
}

#[tokio::test]
async fn completions_endpoint_rejects_body_beyond_explicit_limit(
) -> Result<(), Box<dyn std::error::Error>> {
    let app = router(ServerState::new("fixture-model".to_owned()));
    let request = Request::builder()
        .method("POST")
        .uri("/v1/completions")
        .header("content-type", "application/json")
        .body(Body::from("x".repeat(MAX_OPENAI_REQUEST_BODY_BYTES + 1)))?;
    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_json(response.into_body()).await?;
    assert_eq!(body["error"]["type"], "invalid_request_error");
    let message = body["error"]["message"].as_str().unwrap_or_default();
    assert_eq!(message, "Failed to buffer the request body");
    Ok(())
}

#[tokio::test]
async fn completions_endpoint_returns_openai_error_for_wrong_method(
) -> Result<(), Box<dyn std::error::Error>> {
    let app = router(ServerState::new("fixture-model".to_owned()));
    let request = Request::builder()
        .method("GET")
        .uri("/v1/completions")
        .body(Body::empty())?;
    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    let body = to_json(response.into_body()).await?;
    assert_eq!(body["error"]["type"], "invalid_request_error");
    assert_eq!(body["error"]["code"], "method_not_allowed");
    assert!(body["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("method not allowed"));
    Ok(())
}

#[tokio::test]
async fn unknown_openai_route_returns_openai_error_body() -> Result<(), Box<dyn std::error::Error>>
{
    let app = router(ServerState::new("fixture-model".to_owned()));
    let request = Request::builder()
        .method("GET")
        .uri("/v1/not-a-ferrite-route")
        .body(Body::empty())?;
    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = to_json(response.into_body()).await?;
    assert_eq!(body["error"]["type"], "invalid_request_error");
    assert_eq!(body["error"]["code"], "not_found");
    assert!(body["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("/v1/not-a-ferrite-route"));
    Ok(())
}

async fn to_json(body: Body) -> Result<Value, Box<dyn std::error::Error>> {
    let bytes = to_bytes(body, usize::MAX).await?;
    Ok(serde_json::from_slice(&bytes)?)
}
