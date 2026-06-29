use super::routes::router;
use crate::state::ServerState;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;

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
async fn chat_endpoint_rejects_message_tool_call_fields_without_content(
) -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat(
        r#"{
            "model":"fixture-model",
            "messages":[{
                "role":"assistant",
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
    let message = body.json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("messages.tool_calls"), "{message}");
    assert!(!message.contains("malformed JSON"), "{message}");
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
async fn chat_endpoint_rejects_message_function_call_fields_without_content(
) -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat(
        r#"{
            "model":"fixture-model",
            "messages":[{
                "role":"assistant",
                "function_call":{"name":"lookup","arguments":"{}"}
            }]
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    let message = body.json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("messages.function_call"), "{message}");
    assert!(!message.contains("malformed JSON"), "{message}");
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_function_message_without_name(
) -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat(
        r#"{
            "model":"fixture-model",
            "messages":[{
                "role":"function",
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
async fn chat_endpoint_rejects_tool_call_id_on_non_tool_message(
) -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat(
        r#"{
            "model":"fixture-model",
            "messages":[{
                "role":"user",
                "content":"hello",
                "tool_call_id":"call_1"
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
