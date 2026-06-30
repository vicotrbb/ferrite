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
async fn chat_endpoint_accepts_null_reasoning_effort() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat_json(
        r#"{
            "model":"fixture-model",
            "messages":[{"role":"user","content":"hello"}],
            "reasoning_effort":null
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body.json["error"]["type"], "server_error");
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
