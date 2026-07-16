use super::stream_options_test_support::{post_loaded_chat_stream, post_loaded_completion_stream};
use axum::http::StatusCode;

#[tokio::test]
async fn chat_stream_endpoint_accepts_disabled_obfuscation()
-> Result<(), Box<dyn std::error::Error>> {
    let response = post_loaded_chat_stream(
        r#"{"model":"fixture-model","messages":[{"role":"user","content":"hello"}],"max_completion_tokens":1,"stream":true,"stream_options":{"include_obfuscation":false}}"#,
    )
    .await?;

    assert_eq!(response.status, StatusCode::OK);
    assert!(response.body.contains("\"delta\":{\"content\":\"winner\"}"));
    assert!(
        !response.body.contains("\"obfuscation\":\""),
        "{}",
        response.body
    );
    assert!(response.body.contains("data: [DONE]"));
    Ok(())
}

#[tokio::test]
async fn completion_stream_endpoint_emits_obfuscation_by_default()
-> Result<(), Box<dyn std::error::Error>> {
    let response = post_loaded_completion_stream(
        r#"{"model":"fixture-model","prompt":"hello","stream":true}"#,
    )
    .await?;

    assert_eq!(response.status, StatusCode::OK);
    assert!(response.body.contains("\"text\":\"winner\""));
    assert!(
        response.body.contains("\"obfuscation\":\""),
        "{}",
        response.body
    );
    assert!(response.body.contains("data: [DONE]"));
    Ok(())
}

#[tokio::test]
async fn chat_stream_endpoint_emits_obfuscation_by_default()
-> Result<(), Box<dyn std::error::Error>> {
    let response = post_loaded_chat_stream(
        r#"{"model":"fixture-model","messages":[{"role":"user","content":"hello"}],"max_completion_tokens":1,"stream":true}"#,
    )
    .await?;

    assert_eq!(response.status, StatusCode::OK);
    assert!(response.body.contains("\"delta\":{\"content\":\"winner\"}"));
    assert!(
        response.body.contains("\"obfuscation\":\""),
        "{}",
        response.body
    );
    assert!(response.body.contains("data: [DONE]"));
    Ok(())
}

#[tokio::test]
async fn completion_stream_endpoint_emits_obfuscation_when_requested()
-> Result<(), Box<dyn std::error::Error>> {
    let response = post_loaded_completion_stream(
        r#"{"model":"fixture-model","prompt":"hello","stream":true,"stream_options":{"include_obfuscation":true}}"#,
    )
    .await?;

    assert_eq!(response.status, StatusCode::OK);
    assert!(response.body.contains("\"text\":\"winner\""));
    assert!(response.body.contains("\"obfuscation\":\""));
    assert!(response.body.contains("data: [DONE]"));
    Ok(())
}

#[tokio::test]
async fn chat_stream_endpoint_emits_obfuscation_when_requested()
-> Result<(), Box<dyn std::error::Error>> {
    let response = post_loaded_chat_stream(
        r#"{"model":"fixture-model","messages":[{"role":"user","content":"hello"}],"max_completion_tokens":1,"stream":true,"stream_options":{"include_obfuscation":true}}"#,
    )
    .await?;

    assert_eq!(response.status, StatusCode::OK);
    assert!(response.body.contains("\"delta\":{\"content\":\"winner\"}"));
    assert!(response.body.contains("\"obfuscation\":\""));
    assert!(response.body.contains("data: [DONE]"));
    Ok(())
}
