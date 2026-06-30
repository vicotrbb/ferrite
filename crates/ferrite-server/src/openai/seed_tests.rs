use super::test_support::{post_chat_json, post_completion_json};
use axum::http::StatusCode;

#[tokio::test]
async fn chat_endpoint_accepts_null_seed() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat_json(
        r#"{
            "model":"fixture-model",
            "messages":[{"role":"user","content":"hello"}],
            "seed":null
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body.json["error"]["type"], "server_error");
    Ok(())
}

#[tokio::test]
async fn completion_endpoint_accepts_null_seed() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_completion_json(
        r#"{
            "model":"fixture-model",
            "prompt":"hello",
            "seed":null
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body.json["error"]["type"], "server_error");
    Ok(())
}
