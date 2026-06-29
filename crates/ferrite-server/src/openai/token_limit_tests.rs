use super::routes::router;
use crate::{limits::TokenLimits, state::ServerState};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn chat_endpoint_reports_max_completion_tokens_param_when_hard_limit_is_exceeded(
) -> Result<(), Box<dyn std::error::Error>> {
    let body = post_json(
        "/v1/chat/completions",
        r#"{
            "model":"fixture-model",
            "messages":[{"role":"user","content":"hello"}],
            "max_completion_tokens":3
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    assert_eq!(body.json["error"]["param"], "max_completion_tokens");
    assert!(body.json["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("less than or equal to 2"));
    Ok(())
}

#[tokio::test]
async fn completion_endpoint_reports_max_tokens_param_when_hard_limit_is_exceeded(
) -> Result<(), Box<dyn std::error::Error>> {
    let body = post_json(
        "/v1/completions",
        r#"{
            "model":"fixture-model",
            "prompt":"hello",
            "max_tokens":3
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    assert_eq!(body.json["error"]["param"], "max_tokens");
    assert!(body.json["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("less than or equal to 2"));
    Ok(())
}

struct JsonResponse {
    status: StatusCode,
    json: Value,
}

async fn post_json(
    uri: &'static str,
    payload: &'static str,
) -> Result<JsonResponse, Box<dyn std::error::Error>> {
    let app = router(
        ServerState::new("fixture-model".to_owned()).with_token_limits(TokenLimits::new(1, 2)?),
    );
    let request = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(payload))?;
    let response = app.oneshot(request).await?;
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await?;

    Ok(JsonResponse {
        status,
        json: serde_json::from_slice(&bytes)?,
    })
}
