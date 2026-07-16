use super::test_support::{post_chat_json, post_completion_json};
use axum::http::StatusCode;

#[tokio::test]
async fn chat_endpoint_accepts_null_user_identifier() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat_json(
        r#"{
            "model":"fixture-model",
            "messages":[{"role":"user","content":"hello"}],
            "user":null
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body.json["error"]["type"], "server_error");
    Ok(())
}

#[tokio::test]
async fn completion_endpoint_accepts_null_user_identifier() -> Result<(), Box<dyn std::error::Error>>
{
    let body = post_completion_json(
        r#"{
            "model":"fixture-model",
            "prompt":"hello",
            "user":null
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body.json["error"]["type"], "server_error");
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_malformed_user_identifier() -> Result<(), Box<dyn std::error::Error>>
{
    let body = post_chat_json(
        r#"{
            "model":"fixture-model",
            "messages":[{"role":"user","content":"hello"}],
            "user":{"id":"local-user-1"}
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    assert_eq!(body.json["error"]["param"], "user");
    assert!(
        body.json["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("user")
    );
    Ok(())
}

#[tokio::test]
async fn completion_endpoint_rejects_malformed_user_identifier()
-> Result<(), Box<dyn std::error::Error>> {
    let body = post_completion_json(
        r#"{
            "model":"fixture-model",
            "prompt":"hello",
            "user":123
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    assert_eq!(body.json["error"]["param"], "user");
    assert!(
        body.json["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("user")
    );
    Ok(())
}
