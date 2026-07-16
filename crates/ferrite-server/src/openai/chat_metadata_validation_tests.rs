use super::test_support::post_chat_json;
use axum::http::StatusCode;

#[tokio::test]
async fn chat_endpoint_accepts_null_metadata() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat_json(
        r#"{
            "model":"fixture-model",
            "messages":[{"role":"user","content":"hello"}],
            "metadata":null
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body.json["error"]["type"], "server_error");
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
    assert!(
        body.json["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("metadata")
    );
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
    assert!(
        body.json["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("prompt_cache_key")
    );
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_accepts_null_prompt_cache_key() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat_json(
        r#"{
            "model":"fixture-model",
            "messages":[{"role":"user","content":"hello"}],
            "prompt_cache_key":null
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body.json["error"]["type"], "server_error");
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_malformed_safety_identifier()
-> Result<(), Box<dyn std::error::Error>> {
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
    assert!(
        body.json["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("safety_identifier")
    );
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_accepts_null_safety_identifier() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat_json(
        r#"{
            "model":"fixture-model",
            "messages":[{"role":"user","content":"hello"}],
            "safety_identifier":null
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body.json["error"]["type"], "server_error");
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
    assert!(
        body.json["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("safety_identifier")
    );
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
    assert!(
        body.json["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("seed")
    );
    Ok(())
}
