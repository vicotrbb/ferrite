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
async fn chat_endpoint_rejects_function_fields() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat(
        r#"{
            "model":"fixture-model",
            "messages":[{"role":"user","content":"hello"}],
            "functions":[{"name":"lookup","parameters":{"type":"object"}}],
            "function_call":"auto"
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    let message = body.json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("functions"), "{message}");
    assert!(message.contains("function_call"), "{message}");
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_message_tool_call_fields() -> Result<(), Box<dyn std::error::Error>>
{
    let body = post_chat(
        r#"{
            "model":"fixture-model",
            "messages":[{
                "role":"assistant",
                "content":"hello",
                "tool_calls":[{
                    "id":"call_1",
                    "type":"function",
                    "function":{"name":"lookup","arguments":"{}"}
                }]
            }]
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    assert!(body.json["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("messages.tool_calls"));
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_message_function_call_fields(
) -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat(
        r#"{
            "model":"fixture-model",
            "messages":[{
                "role":"assistant",
                "content":"hello",
                "function_call":{"name":"lookup","arguments":"{}"}
            }]
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    assert!(body.json["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("messages.function_call"));
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_unknown_message_fields() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat(
        r#"{
            "model":"fixture-model",
            "messages":[{
                "role":"user",
                "content":"hello",
                "vendor_context":{"trace":"local"}
            }]
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    assert!(body.json["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("messages.vendor_context"));
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_malformed_message_name() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat(
        r#"{
            "model":"fixture-model",
            "messages":[{
                "role":"user",
                "content":"hello",
                "name":{"id":"local-user"}
            }]
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    assert!(body.json["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("messages.name"));
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_malformed_message_tool_call_id(
) -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat(
        r#"{
            "model":"fixture-model",
            "messages":[{
                "role":"tool",
                "content":"hello",
                "tool_call_id":123
            }]
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    assert!(body.json["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("messages.tool_call_id"));
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_tool_message_without_tool_call_id(
) -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat(
        r#"{
            "model":"fixture-model",
            "messages":[{
                "role":"tool",
                "content":"hello"
            }]
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    assert!(body.json["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("messages.tool_call_id"));
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_json_response_format() -> Result<(), Box<dyn std::error::Error>> {
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
async fn chat_endpoint_rejects_sampling_parameters() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat(
        r#"{
            "model":"fixture-model",
            "messages":[{"role":"user","content":"hello"}],
            "temperature":0.2
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    assert!(body.json["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("temperature"));
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_enabled_reasoning_effort() -> Result<(), Box<dyn std::error::Error>>
{
    let body = post_chat(
        r#"{
            "model":"fixture-model",
            "messages":[{"role":"user","content":"hello"}],
            "reasoning_effort":"low"
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    assert!(body.json["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("reasoning_effort"));
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_multiple_choice_request() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat(
        r#"{
            "model":"fixture-model",
            "messages":[{"role":"user","content":"hello"}],
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
async fn chat_endpoint_rejects_malformed_metadata() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat(
        r#"{
            "model":"fixture-model",
            "messages":[{"role":"user","content":"hello"}],
            "metadata":{"trace_id":123}
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    assert!(body.json["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("metadata"));
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_malformed_prompt_cache_key() -> Result<(), Box<dyn std::error::Error>>
{
    let body = post_chat(
        r#"{
            "model":"fixture-model",
            "messages":[{"role":"user","content":"hello"}],
            "prompt_cache_key":123
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    assert!(body.json["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("prompt_cache_key"));
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_malformed_safety_identifier(
) -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat(
        r#"{
            "model":"fixture-model",
            "messages":[{"role":"user","content":"hello"}],
            "safety_identifier":123
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    assert!(body.json["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("safety_identifier"));
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_overlong_safety_identifier() -> Result<(), Box<dyn std::error::Error>>
{
    let safety_identifier = "s".repeat(65);
    let payload = format!(
        r#"{{
            "model":"fixture-model",
            "messages":[{{"role":"user","content":"hello"}}],
            "safety_identifier":"{safety_identifier}"
        }}"#
    );
    let body = post_chat(&payload).await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    assert!(body.json["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("safety_identifier"));
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_malformed_seed() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat(
        r#"{
            "model":"fixture-model",
            "messages":[{"role":"user","content":"hello"}],
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
async fn chat_endpoint_rejects_unknown_fields() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat(
        r#"{
            "model":"fixture-model",
            "messages":[{"role":"user","content":"hello"}],
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

async fn post_chat(payload: &str) -> Result<JsonResponse, Box<dyn std::error::Error>> {
    let app = router(ServerState::new("fixture-model".to_owned()));
    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_owned()))?;
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
