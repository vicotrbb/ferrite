use super::routes::router;
use crate::state::ServerState;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;

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
    assert_eq!(body.json["error"]["param"], "logprobs");
    assert!(body.json["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("logprobs"));
    Ok(())
}

#[tokio::test]
async fn completion_endpoint_rejects_missing_model() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_completion(
        r#"{
            "prompt":"hello"
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    let message = body.json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("model"), "{message}");
    assert!(!message.contains("malformed JSON"), "{message}");
    Ok(())
}

#[tokio::test]
async fn completion_endpoint_rejects_null_model() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_completion(
        r#"{
            "model":null,
            "prompt":"hello"
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    let message = body.json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("model"), "{message}");
    assert!(!message.contains("malformed JSON"), "{message}");
    Ok(())
}

#[tokio::test]
async fn completion_endpoint_rejects_missing_prompt() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_completion(
        r#"{
            "model":"fixture-model"
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    let message = body.json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("prompt"), "{message}");
    assert!(!message.contains("malformed JSON"), "{message}");
    Ok(())
}

#[tokio::test]
async fn completion_endpoint_rejects_null_prompt() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_completion(
        r#"{
            "model":"fixture-model",
            "prompt":null
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    let message = body.json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("prompt"), "{message}");
    assert!(!message.contains("malformed JSON"), "{message}");
    Ok(())
}

#[tokio::test]
async fn completion_endpoint_rejects_object_prompt() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_completion(
        r#"{
            "model":"fixture-model",
            "prompt":{"text":"hello"}
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    let message = body.json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("prompt"), "{message}");
    assert!(!message.contains("malformed JSON"), "{message}");
    Ok(())
}

#[tokio::test]
async fn completion_endpoint_rejects_malformed_seed() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_completion(
        r#"{
            "model":"fixture-model",
            "prompt":"hello",
            "seed":"42"
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    assert!(body.json["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("seed"));
    Ok(())
}

#[tokio::test]
async fn completion_endpoint_rejects_malformed_max_tokens() -> Result<(), Box<dyn std::error::Error>>
{
    let body = post_completion(
        r#"{
            "model":"fixture-model",
            "prompt":"hello",
            "max_tokens":"1"
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    let message = body.json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("max_tokens"), "{message}");
    assert!(!message.contains("malformed JSON"), "{message}");
    Ok(())
}

#[tokio::test]
async fn completion_endpoint_rejects_malformed_stream_flag(
) -> Result<(), Box<dyn std::error::Error>> {
    let body = post_completion(
        r#"{
            "model":"fixture-model",
            "prompt":"hello",
            "stream":"yes"
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    let message = body.json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("stream"), "{message}");
    assert!(!message.contains("malformed JSON"), "{message}");
    Ok(())
}

#[tokio::test]
async fn completion_endpoint_rejects_token_prompt_array() -> Result<(), Box<dyn std::error::Error>>
{
    let body = post_completion(
        r#"{
            "model":"fixture-model",
            "prompt":[15496,995]
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    let message = body.json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("prompt"), "{message}");
    assert!(!message.contains("malformed JSON"), "{message}");
    Ok(())
}

#[tokio::test]
async fn completion_endpoint_rejects_token_prompt_array_batch(
) -> Result<(), Box<dyn std::error::Error>> {
    let body = post_completion(
        r#"{
            "model":"fixture-model",
            "prompt":[[15496,995]]
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    let message = body.json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("prompt"), "{message}");
    assert!(!message.contains("malformed JSON"), "{message}");
    Ok(())
}

#[tokio::test]
async fn completion_endpoint_rejects_unknown_fields() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_completion(
        r#"{
            "model":"fixture-model",
            "prompt":"hello",
            "unsupported_option":true
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    assert!(body.json["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("unsupported_option"));
    Ok(())
}

struct JsonResponse {
    status: StatusCode,
    json: Value,
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
