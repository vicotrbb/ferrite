use super::routes::router;
use crate::state::ServerState;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn chat_endpoint_rejects_tool_fields() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat(
        r#"{
            "model":"fixture-model",
            "messages":[{"role":"user","content":"hello"}],
            "tools":[{"type":"function","function":{"name":"lookup","parameters":{"type":"object"}}}]
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    assert!(body.json["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("tools"));
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_response_format() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat(
        r#"{
            "model":"fixture-model",
            "messages":[{"role":"user","content":"hello"}],
            "response_format":{"type":"json_object"}
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    assert!(body.json["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("response_format"));
    Ok(())
}

#[tokio::test]
async fn completion_endpoint_rejects_multiple_choice_request(
) -> Result<(), Box<dyn std::error::Error>> {
    let body = post_completion(
        r#"{
            "model":"fixture-model",
            "prompt":"hello",
            "n":2
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    assert!(body.json["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("n"));
    Ok(())
}

#[tokio::test]
async fn completion_endpoint_rejects_logprobs_request() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_completion(
        r#"{
            "model":"fixture-model",
            "prompt":"hello",
            "logprobs":5
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    assert!(body.json["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("logprobs"));
    Ok(())
}

struct JsonResponse {
    status: StatusCode,
    json: Value,
}

async fn post_chat(payload: &'static str) -> Result<JsonResponse, Box<dyn std::error::Error>> {
    let app = router(ServerState::new("fixture-model".to_owned()));
    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
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

async fn post_completion(
    payload: &'static str,
) -> Result<JsonResponse, Box<dyn std::error::Error>> {
    let app = router(ServerState::new("fixture-model".to_owned()));
    let request = Request::builder()
        .method("POST")
        .uri("/v1/completions")
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
