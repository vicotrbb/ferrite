use super::test_support::post_completion_json;
use axum::http::StatusCode;

#[tokio::test]
async fn completion_endpoint_rejects_missing_prompt() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_completion_json(
        r#"{
            "model":"fixture-model"
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    let message = body.json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("prompt"), "{message}");
    assert!(!message.contains("malformed JSON"), "{message}");
    Ok(())
}

#[tokio::test]
async fn completion_endpoint_rejects_null_prompt() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_completion_json(
        r#"{
            "model":"fixture-model",
            "prompt":null
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    let message = body.json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("prompt"), "{message}");
    assert!(!message.contains("malformed JSON"), "{message}");
    Ok(())
}

#[tokio::test]
async fn completion_endpoint_rejects_object_prompt() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_completion_json(
        r#"{
            "model":"fixture-model",
            "prompt":{"text":"hello"}
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    let message = body.json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("prompt"), "{message}");
    assert!(!message.contains("malformed JSON"), "{message}");
    Ok(())
}

#[tokio::test]
async fn completion_endpoint_rejects_token_prompt_array() -> Result<(), Box<dyn std::error::Error>>
{
    let body = post_completion_json(
        r#"{
            "model":"fixture-model",
            "prompt":[15496,995]
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    let message = body.json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("prompt"), "{message}");
    assert!(!message.contains("malformed JSON"), "{message}");
    Ok(())
}

#[tokio::test]
async fn completion_endpoint_rejects_token_prompt_array_batch(
) -> Result<(), Box<dyn std::error::Error>> {
    let body = post_completion_json(
        r#"{
            "model":"fixture-model",
            "prompt":[[15496,995]]
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    let message = body.json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("prompt"), "{message}");
    assert!(!message.contains("malformed JSON"), "{message}");
    Ok(())
}

#[tokio::test]
async fn completion_endpoint_reports_prompt_param_for_empty_or_whitespace_prompt(
) -> Result<(), Box<dyn std::error::Error>> {
    for (payload, expected_message) in [
        (
            r#"{
                "model":"fixture-model",
                "prompt":[]
            }"#,
            "prompt must contain at least one item",
        ),
        (
            r#"{
                "model":"fixture-model",
                "prompt":"   "
            }"#,
            "prompt must contain non-whitespace text",
        ),
    ] {
        let body = post_completion_json(payload).await?;

        assert_eq!(body.status, StatusCode::BAD_REQUEST);
        assert_eq!(body.json["error"]["type"], "invalid_request_error");
        assert_eq!(body.json["error"]["param"], "prompt", "{}", body.json);
        let message = body.json["error"]["message"].as_str().unwrap_or_default();
        assert!(message.contains(expected_message), "{message}");
    }
    Ok(())
}
