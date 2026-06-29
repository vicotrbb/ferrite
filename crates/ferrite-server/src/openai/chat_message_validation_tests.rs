use super::test_support::post_chat_json;
use axum::http::StatusCode;

#[tokio::test]
async fn chat_endpoint_rejects_assistant_audio_object() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat_json(
        r#"{
            "model":"fixture-model",
            "messages":[{
                "role":"assistant",
                "content":"hello",
                "audio":{"id":"audio_1"}
            }]
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    assert!(body.json["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("messages.audio"));
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_assistant_refusal_metadata_string(
) -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat_json(
        r#"{
            "model":"fixture-model",
            "messages":[{
                "role":"assistant",
                "content":"hello",
                "refusal":"blocked"
            }]
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    assert!(body.json["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("messages.refusal"));
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_unknown_message_fields() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat_json(
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
async fn chat_endpoint_rejects_unknown_message_role() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat_json(
        r#"{
            "model":"fixture-model",
            "messages":[{
                "role":"critic",
                "content":"hello"
            }]
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    let message = body.json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("messages.role"), "{message}");
    assert!(!message.contains("malformed JSON"), "{message}");
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_non_string_message_role() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat_json(
        r#"{
            "model":"fixture-model",
            "messages":[{
                "role":42,
                "content":"hello"
            }]
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    let message = body.json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("messages.role"), "{message}");
    assert!(!message.contains("malformed JSON"), "{message}");
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_message_without_role() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat_json(
        r#"{
            "model":"fixture-model",
            "messages":[{
                "content":"hello"
            }]
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    let message = body.json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("messages.role"), "{message}");
    assert!(!message.contains("malformed JSON"), "{message}");
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_message_without_content() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat_json(
        r#"{
            "model":"fixture-model",
            "messages":[{
                "role":"user"
            }]
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    let message = body.json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("messages.content"), "{message}");
    assert!(!message.contains("malformed JSON"), "{message}");
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_malformed_message_content() -> Result<(), Box<dyn std::error::Error>>
{
    let body = post_chat_json(
        r#"{
            "model":"fixture-model",
            "messages":[{
                "role":"user",
                "content":42
            }]
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    let message = body.json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("messages.content"), "{message}");
    assert!(!message.contains("malformed JSON"), "{message}");
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_user_refusal_content_parts() -> Result<(), Box<dyn std::error::Error>>
{
    let body = post_chat_json(
        r#"{
            "model":"fixture-model",
            "messages":[{
                "role":"user",
                "content":[{"type":"refusal","refusal":"blocked"}]
            }]
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    let message = body.json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("messages.content"), "{message}");
    assert!(!message.contains("malformed JSON"), "{message}");
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_image_content_parts() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat_json(
        r#"{
            "model":"fixture-model",
            "messages":[{
                "role":"user",
                "content":[{
                    "type":"image_url",
                    "image_url":{"url":"https://example.test/image.png"}
                }]
            }]
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    let message = body.json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("messages.content"), "{message}");
    assert!(!message.contains("malformed JSON"), "{message}");
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_audio_content_parts() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat_json(
        r#"{
            "model":"fixture-model",
            "messages":[{
                "role":"user",
                "content":[{
                    "type":"input_audio",
                    "input_audio":{"data":"ZmVycml0ZQ==","format":"wav"}
                }]
            }]
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    let message = body.json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("messages.content"), "{message}");
    assert!(!message.contains("malformed JSON"), "{message}");
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_malformed_text_content_parts(
) -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat_json(
        r#"{
            "model":"fixture-model",
            "messages":[{
                "role":"user",
                "content":[{"type":"text"}]
            }]
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    let message = body.json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("messages.content"), "{message}");
    assert!(!message.contains("malformed JSON"), "{message}");
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_non_string_text_content_parts(
) -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat_json(
        r#"{
            "model":"fixture-model",
            "messages":[{
                "role":"user",
                "content":[{"type":"text","text":42}]
            }]
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    let message = body.json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("messages.content"), "{message}");
    assert!(!message.contains("malformed JSON"), "{message}");
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_malformed_message_name() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat_json(
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
