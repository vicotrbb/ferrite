use super::routes::router;
use crate::state::ServerState;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn model_retrieve_endpoint_returns_loaded_model() -> Result<(), Box<dyn std::error::Error>> {
    let app = router(ServerState::new("test-model".to_owned()));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/models/test-model")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_json(response.into_body()).await?;
    assert_eq!(body["id"], "test-model");
    assert_eq!(body["object"], "model");
    assert_eq!(body["owned_by"], "ferrite");
    Ok(())
}

#[tokio::test]
async fn model_retrieve_endpoint_rejects_unknown_model() -> Result<(), Box<dyn std::error::Error>> {
    let app = router(ServerState::new("test-model".to_owned()));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/models/other-model")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = to_json(response.into_body()).await?;
    assert_eq!(body["error"]["type"], "invalid_request_error");
    assert_eq!(body["error"]["code"], "model_not_found");
    Ok(())
}

async fn to_json(body: Body) -> Result<Value, Box<dyn std::error::Error>> {
    let bytes = to_bytes(body, usize::MAX).await?;
    Ok(serde_json::from_slice(&bytes)?)
}
