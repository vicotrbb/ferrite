use super::routes::router;
use super::test_support::{remove_fixture_model, write_chat_fixture_model};
use crate::{limits::TokenLimits, runtime::InferenceEngine, state::ServerState};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn chat_endpoint_honors_max_completion_tokens() -> Result<(), Box<dyn std::error::Error>> {
    let model_path = write_chat_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let app = router(ServerState::with_engine("fixture-model".to_owned(), engine));
    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"model":"fixture-model","messages":[{"role":"user","content":"hello"}],"max_completion_tokens":1}"#,
        ))?;
    let response = app.oneshot(request).await?;
    remove_fixture_model(&model_path)?;

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_json(response.into_body()).await?;
    assert_eq!(body["choices"][0]["message"]["content"], "winner");
    assert_eq!(body["usage"]["completion_tokens"], 1);
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_uses_configured_default_max_tokens() -> Result<(), Box<dyn std::error::Error>>
{
    let model_path = write_chat_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let app = router(
        ServerState::with_engine("fixture-model".to_owned(), engine)
            .with_token_limits(TokenLimits::new(1, 8)?),
    );
    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"model":"fixture-model","messages":[{"role":"user","content":"hello"}]}"#,
        ))?;
    let response = app.oneshot(request).await?;
    remove_fixture_model(&model_path)?;

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_json(response.into_body()).await?;
    assert_eq!(body["choices"][0]["message"]["content"], "winner");
    assert_eq!(body["usage"]["completion_tokens"], 1);
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_configured_hard_max_tokens() -> Result<(), Box<dyn std::error::Error>>
{
    let app = router(
        ServerState::new("fixture-model".to_owned()).with_token_limits(TokenLimits::new(1, 2)?),
    );
    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"model":"fixture-model","messages":[{"role":"user","content":"hello"}],"max_tokens":3}"#,
        ))?;
    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_json(response.into_body()).await?;
    assert_eq!(body["error"]["type"], "invalid_request_error");
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("less than or equal to 2")
    );
    Ok(())
}

#[tokio::test]
async fn chat_stream_endpoint_honors_max_completion_tokens()
-> Result<(), Box<dyn std::error::Error>> {
    let model_path = write_chat_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let app = router(ServerState::with_engine("fixture-model".to_owned(), engine));
    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"model":"fixture-model","messages":[{"role":"user","content":"hello"}],"max_completion_tokens":1,"stream":true}"#,
        ))?;
    let response = app.oneshot(request).await?;
    remove_fixture_model(&model_path)?;

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_text(response.into_body()).await?;
    assert!(body.contains("\"delta\":{\"content\":\"winner\"}"));
    assert_eq!(
        body.matches("\"delta\":{\"content\":\"winner\"}").count(),
        1
    );
    assert!(body.contains("data: [DONE]"));
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_reports_max_completion_tokens_param_when_hard_limit_is_exceeded()
-> Result<(), Box<dyn std::error::Error>> {
    let body = post_json(
        "/v1/chat/completions",
        r#"{
            "model":"fixture-model",
            "messages":[{"role":"user","content":"hello"}],
            "max_completion_tokens":3
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    assert_eq!(body.json["error"]["param"], "max_completion_tokens");
    let message = body.json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("max_completion_tokens"), "{message}");
    assert!(message.contains("less than or equal to 2"), "{message}");
    assert!(!message.contains("max_tokens must"), "{message}");
    Ok(())
}

#[tokio::test]
async fn completion_endpoint_reports_max_tokens_param_when_hard_limit_is_exceeded()
-> Result<(), Box<dyn std::error::Error>> {
    let body = post_json(
        "/v1/completions",
        r#"{
            "model":"fixture-model",
            "prompt":"hello",
            "max_tokens":3
        }"#,
    )
    .await?;

    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body.json["error"]["type"], "invalid_request_error");
    assert_eq!(body.json["error"]["param"], "max_tokens");
    assert!(
        body.json["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("less than or equal to 2")
    );
    Ok(())
}

#[tokio::test]
async fn endpoints_report_token_limit_param_when_zero_is_requested()
-> Result<(), Box<dyn std::error::Error>> {
    for (uri, payload, param) in [
        (
            "/v1/chat/completions",
            r#"{
                "model":"fixture-model",
                "messages":[{"role":"user","content":"hello"}],
                "max_completion_tokens":0
            }"#,
            "max_completion_tokens",
        ),
        (
            "/v1/completions",
            r#"{
                "model":"fixture-model",
                "prompt":"hello",
                "max_tokens":0
            }"#,
            "max_tokens",
        ),
    ] {
        let body = post_json(uri, payload).await?;

        assert_eq!(body.status, StatusCode::BAD_REQUEST);
        assert_eq!(body.json["error"]["type"], "invalid_request_error");
        assert_eq!(body.json["error"]["param"], param, "{}", body.json);
        let message = body.json["error"]["message"].as_str().unwrap_or_default();
        assert!(message.contains(param), "{message}");
        assert!(message.contains("greater than zero"), "{message}");
    }
    Ok(())
}

struct JsonResponse {
    status: StatusCode,
    json: Value,
}

async fn post_json(
    uri: &'static str,
    payload: &'static str,
) -> Result<JsonResponse, Box<dyn std::error::Error>> {
    let app = router(
        ServerState::new("fixture-model".to_owned()).with_token_limits(TokenLimits::new(1, 2)?),
    );
    let request = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(payload))?;
    let response = app.oneshot(request).await?;
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await?;

    Ok(JsonResponse {
        status,
        json: serde_json::from_slice(&bytes)?,
    })
}

async fn to_json(body: Body) -> Result<Value, Box<dyn std::error::Error>> {
    let bytes = to_bytes(body, usize::MAX).await?;
    Ok(serde_json::from_slice(&bytes)?)
}

async fn to_text(body: Body) -> Result<String, Box<dyn std::error::Error>> {
    let bytes = to_bytes(body, usize::MAX).await?;
    Ok(String::from_utf8(bytes.to_vec())?)
}
