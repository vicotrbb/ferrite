use super::test_support::post_completion_json;
use axum::http::StatusCode;

#[tokio::test]
async fn completion_endpoint_rejects_multiple_choice_request()
-> Result<(), Box<dyn std::error::Error>> {
    let body = post_completion_json(
        r#"{
            "model":"fixture-model",
            "prompt":"hello",
            "n":2
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    assert!(
        body.json["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("n")
    );
    Ok(())
}

#[tokio::test]
async fn completion_endpoint_rejects_multiple_best_of_candidates()
-> Result<(), Box<dyn std::error::Error>> {
    let body = post_completion_json(
        r#"{
            "model":"fixture-model",
            "prompt":"hello",
            "best_of":2
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    assert_eq!(body.json["error"]["param"], "best_of");
    assert!(
        body.json["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("best_of")
    );
    Ok(())
}

#[tokio::test]
async fn completion_endpoint_rejects_malformed_echo_request()
-> Result<(), Box<dyn std::error::Error>> {
    let body = post_completion_json(
        r#"{
            "model":"fixture-model",
            "prompt":"hello",
            "echo":"yes"
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    assert_eq!(body.json["error"]["param"], "echo");
    assert!(
        body.json["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("echo")
    );
    Ok(())
}

#[tokio::test]
async fn completion_endpoint_rejects_logprobs_request() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_completion_json(
        r#"{
            "model":"fixture-model",
            "prompt":"hello",
            "logprobs":5
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    assert_eq!(body.json["error"]["param"], "logprobs");
    assert!(
        body.json["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("logprobs")
    );
    Ok(())
}

#[tokio::test]
async fn completion_endpoint_rejects_missing_model() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_completion_json(
        r#"{
            "prompt":"hello"
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
async fn completion_endpoint_rejects_null_model() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_completion_json(
        r#"{
            "model":null,
            "prompt":"hello"
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
async fn completion_endpoint_rejects_malformed_seed() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_completion_json(
        r#"{
            "model":"fixture-model",
            "prompt":"hello",
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

#[tokio::test]
async fn completion_endpoint_rejects_malformed_max_tokens() -> Result<(), Box<dyn std::error::Error>>
{
    let body = post_completion_json(
        r#"{
            "model":"fixture-model",
            "prompt":"hello",
            "max_tokens":"1"
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    let message = body.json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("max_tokens"), "{message}");
    assert!(!message.contains("malformed JSON"), "{message}");
    Ok(())
}

#[tokio::test]
async fn completion_endpoint_rejects_malformed_stream_flag()
-> Result<(), Box<dyn std::error::Error>> {
    let body = post_completion_json(
        r#"{
            "model":"fixture-model",
            "prompt":"hello",
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
async fn completion_endpoint_rejects_unknown_fields() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_completion_json(
        r#"{
            "model":"fixture-model",
            "prompt":"hello",
            "unsupported_option":true
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    assert!(
        body.json["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("unsupported_option")
    );
    Ok(())
}
