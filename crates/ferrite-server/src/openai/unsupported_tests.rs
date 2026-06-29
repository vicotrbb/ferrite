use super::test_support::post_chat_json;
use axum::http::StatusCode;

#[tokio::test]
async fn chat_endpoint_rejects_tool_fields() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat_json(
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
async fn chat_endpoint_rejects_missing_model() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat_json(
        r#"{
            "messages":[{"role":"user","content":"hello"}]
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
async fn chat_endpoint_rejects_non_string_model() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat_json(
        r#"{
            "model":42,
            "messages":[{"role":"user","content":"hello"}]
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
async fn chat_endpoint_rejects_missing_messages() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat_json(
        r#"{
            "model":"fixture-model"
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    let message = body.json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("messages"), "{message}");
    assert!(!message.contains("malformed JSON"), "{message}");
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_null_messages() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat_json(
        r#"{
            "model":"fixture-model",
            "messages":null
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    let message = body.json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("messages"), "{message}");
    assert!(!message.contains("malformed JSON"), "{message}");
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_non_array_messages() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat_json(
        r#"{
            "model":"fixture-model",
            "messages":"hello"
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    let message = body.json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("messages"), "{message}");
    assert!(!message.contains("malformed JSON"), "{message}");
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_non_object_message_items() -> Result<(), Box<dyn std::error::Error>>
{
    let body = post_chat_json(
        r#"{
            "model":"fixture-model",
            "messages":[42]
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    let message = body.json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("messages.role"), "{message}");
    assert!(message.contains("messages.content"), "{message}");
    assert!(
        !message.contains("messages must contain at least one item"),
        "{message}"
    );
    assert!(!message.contains("malformed JSON"), "{message}");
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_function_fields() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat_json(
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
async fn chat_endpoint_rejects_json_response_format() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat_json(
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
    let body = post_chat_json(
        r#"{
            "model":"fixture-model",
            "messages":[{"role":"user","content":"hello"}],
            "temperature":0.2
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    assert_eq!(body.json["error"]["param"], "temperature");
    assert!(body.json["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("temperature"));
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_enabled_reasoning_effort() -> Result<(), Box<dyn std::error::Error>>
{
    let body = post_chat_json(
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
    let body = post_chat_json(
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
    let body = post_chat_json(
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
    let body = post_chat_json(
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
    let body = post_chat_json(
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
    let body = post_chat_json(&payload).await?;

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
    let body = post_chat_json(
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
async fn chat_endpoint_rejects_malformed_max_completion_tokens(
) -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat_json(
        r#"{
            "model":"fixture-model",
            "messages":[{"role":"user","content":"hello"}],
            "max_completion_tokens":"1"
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    let message = body.json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("max_completion_tokens"), "{message}");
    assert!(!message.contains("malformed JSON"), "{message}");
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_malformed_stream_flag() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat_json(
        r#"{
            "model":"fixture-model",
            "messages":[{"role":"user","content":"hello"}],
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
async fn chat_endpoint_rejects_unknown_fields() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat_json(
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
