use super::stream_options_test_support::{post_unloaded_chat, post_unloaded_completion};
use axum::http::StatusCode;

#[tokio::test]
async fn chat_endpoint_rejects_stream_options_without_streaming()
-> Result<(), Box<dyn std::error::Error>> {
    let response = post_unloaded_chat(
        r#"{"model":"fixture-model","messages":[{"role":"user","content":"hello"}],"stream_options":{"include_usage":true}}"#,
    )
    .await?;

    assert_eq!(response.status, StatusCode::BAD_REQUEST);
    assert!(response.body.contains("stream_options"));
    Ok(())
}

#[tokio::test]
async fn completion_endpoint_rejects_stream_options_without_streaming()
-> Result<(), Box<dyn std::error::Error>> {
    let response = post_unloaded_completion(
        r#"{"model":"fixture-model","prompt":"hello","stream_options":{"include_obfuscation":false}}"#,
    )
    .await?;

    assert_eq!(response.status, StatusCode::BAD_REQUEST);
    assert!(response.body.contains("stream_options"));
    Ok(())
}

#[tokio::test]
async fn chat_stream_endpoint_rejects_malformed_include_usage()
-> Result<(), Box<dyn std::error::Error>> {
    let response = post_unloaded_chat(
        r#"{"model":"fixture-model","messages":[{"role":"user","content":"hello"}],"stream":true,"stream_options":{"include_usage":"yes"}}"#,
    )
    .await?;

    assert_eq!(response.status, StatusCode::BAD_REQUEST);
    assert!(response.body.contains("stream_options.include_usage"));
    assert!(!response.body.contains("malformed JSON"));
    Ok(())
}

#[tokio::test]
async fn completions_stream_endpoint_rejects_malformed_include_usage()
-> Result<(), Box<dyn std::error::Error>> {
    let response = post_unloaded_completion(
        r#"{"model":"fixture-model","prompt":"hello","stream":true,"stream_options":{"include_usage":"yes"}}"#,
    )
    .await?;

    assert_eq!(response.status, StatusCode::BAD_REQUEST);
    assert!(response.body.contains("stream_options.include_usage"));
    assert!(!response.body.contains("malformed JSON"));
    Ok(())
}
