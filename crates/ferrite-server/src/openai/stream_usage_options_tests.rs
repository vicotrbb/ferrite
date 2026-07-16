use super::stream_options_test_support::{post_loaded_chat_stream, post_loaded_completion_stream};
use axum::http::StatusCode;

#[tokio::test]
async fn completions_stream_endpoint_emits_usage_when_requested()
-> Result<(), Box<dyn std::error::Error>> {
    let response = post_loaded_completion_stream(
        r#"{"model":"fixture-model","prompt":"hello","max_tokens":1,"stream":true,"stream_options":{"include_usage":true}}"#,
    )
    .await?;

    assert_eq!(response.status, StatusCode::OK);
    assert!(response.body.contains("\"text\":\"winner\""));
    assert!(response.body.contains("\"usage\":null"));
    assert!(response.body.contains("\"choices\":[]"));
    assert!(response.body.contains("\"usage\":{\"prompt_tokens\":"));
    assert!(response.body.contains("\"completion_tokens\":1"));
    assert!(response.body.contains("\"total_tokens\":"));
    assert!(response.body.contains("data: [DONE]"));
    Ok(())
}

#[tokio::test]
async fn chat_stream_endpoint_emits_usage_when_requested() -> Result<(), Box<dyn std::error::Error>>
{
    let response = post_loaded_chat_stream(
        r#"{"model":"fixture-model","messages":[{"role":"user","content":"hello"}],"max_completion_tokens":1,"stream":true,"stream_options":{"include_usage":true}}"#,
    )
    .await?;

    assert_eq!(response.status, StatusCode::OK);
    assert!(response.body.contains("\"delta\":{\"content\":\"winner\"}"));
    assert!(response.body.contains("\"usage\":null"));
    assert!(response.body.contains("\"choices\":[]"));
    assert!(response.body.contains("\"usage\":{\"prompt_tokens\":"));
    assert!(response.body.contains("\"completion_tokens\":1"));
    assert!(response.body.contains("\"total_tokens\":"));
    assert!(response.body.contains("data: [DONE]"));
    Ok(())
}
