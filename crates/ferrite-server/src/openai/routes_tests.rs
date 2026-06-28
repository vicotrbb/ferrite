use super::routes::router;
use crate::{runtime::InferenceEngine, state::ServerState};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use tower::ServiceExt;

static FIXTURE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[tokio::test]
async fn health_endpoint_reports_ready_model() -> Result<(), Box<dyn std::error::Error>> {
    let app = router(ServerState::new("test-model".to_owned()));
    let response = app
        .oneshot(Request::builder().uri("/health").body(Body::empty())?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_json(response.into_body()).await?;
    assert_eq!(body["status"], "ok");
    assert_eq!(body["model"], "test-model");
    assert_eq!(body["ready"], true);
    Ok(())
}

#[tokio::test]
async fn models_endpoint_returns_openai_list_shape() -> Result<(), Box<dyn std::error::Error>> {
    let app = router(ServerState::new("test-model".to_owned()));
    let response = app
        .oneshot(Request::builder().uri("/v1/models").body(Body::empty())?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_json(response.into_body()).await?;
    assert_eq!(body["object"], "list");
    assert_eq!(body["data"][0]["id"], "test-model");
    assert_eq!(body["data"][0]["object"], "model");
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_returns_openai_error_when_model_is_not_loaded(
) -> Result<(), Box<dyn std::error::Error>> {
    let app = router(ServerState::new("test-model".to_owned()));
    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"model":"test-model","messages":[{"role":"user","content":"Hello"}]}"#,
        ))?;
    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = to_json(response.into_body()).await?;
    assert_eq!(body["error"]["type"], "server_error");
    Ok(())
}

#[tokio::test]
async fn completions_endpoint_generates_with_loaded_fixture_model(
) -> Result<(), Box<dyn std::error::Error>> {
    let model_path = write_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let app = router(ServerState::with_engine("fixture-model".to_owned(), engine));
    let request = Request::builder()
        .method("POST")
        .uri("/v1/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"model":"fixture-model","prompt":"hello","max_tokens":1}"#,
        ))?;
    let response = app.oneshot(request).await?;
    remove_fixture_model(&model_path)?;

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_json(response.into_body()).await?;
    assert_eq!(body["object"], "text_completion");
    assert_eq!(body["model"], "fixture-model");
    assert_eq!(body["choices"][0]["text"], "winner");
    assert_eq!(body["usage"]["prompt_tokens"], 1);
    assert_eq!(body["usage"]["completion_tokens"], 1);
    assert_eq!(body["usage"]["total_tokens"], 2);
    Ok(())
}

#[tokio::test]
async fn completions_endpoint_streams_openai_sse_chunks() -> Result<(), Box<dyn std::error::Error>>
{
    let model_path = write_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let app = router(ServerState::with_engine("fixture-model".to_owned(), engine));
    let request = Request::builder()
        .method("POST")
        .uri("/v1/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"model":"fixture-model","prompt":"hello","max_tokens":1,"stream":true}"#,
        ))?;
    let response = app.oneshot(request).await?;
    remove_fixture_model(&model_path)?;

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_text(response.into_body()).await?;
    assert!(body.contains("data: {\"id\":\"cmpl-ferrite-"));
    assert!(body.contains("\"object\":\"text_completion\""));
    assert!(body.contains("\"text\":\"winner\""));
    assert!(body.contains("data: [DONE]"));
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_streams_openai_sse_chunks() -> Result<(), Box<dyn std::error::Error>> {
    let model_path = write_chat_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let app = router(ServerState::with_engine("fixture-model".to_owned(), engine));
    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"model":"fixture-model","messages":[{"role":"user","content":"hello"}],"max_tokens":1,"stream":true}"#,
        ))?;
    let response = app.oneshot(request).await?;
    remove_fixture_model(&model_path)?;

    let status = response.status();
    let body = to_text(response.into_body()).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body.contains("data: {\"id\":\"chatcmpl-ferrite-"));
    assert!(body.contains("\"object\":\"chat.completion.chunk\""));
    assert!(body.contains("\"delta\":{\"content\":\"winner\"}"));
    assert!(body.contains("data: [DONE]"));
    Ok(())
}

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
            .with_token_limits(crate::limits::TokenLimits::new(1, 8)?),
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
        ServerState::new("fixture-model".to_owned())
            .with_token_limits(crate::limits::TokenLimits::new(1, 2)?),
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
    assert!(body["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("less than or equal to 2"));
    Ok(())
}

#[tokio::test]
async fn chat_stream_endpoint_honors_max_completion_tokens(
) -> Result<(), Box<dyn std::error::Error>> {
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
async fn completion_stream_helper_emits_tokens_from_generation_callback(
) -> Result<(), Box<dyn std::error::Error>> {
    let model_path = write_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    remove_fixture_model(&model_path)?;
    let state = ServerState::with_engine("fixture-model".to_owned(), engine);
    let permit = state
        .try_acquire_inference_permit()
        .ok_or("expected inference permit")?;

    let response = super::routes::completion_stream_response(
        state.engine().ok_or("expected inference engine")?,
        "fixture-model".to_owned(),
        "hello".to_owned(),
        1,
        permit,
    );
    let body = to_text(response.into_body()).await?;

    assert!(body.contains("\"text\":\"winner\""));
    assert!(body.contains("data: [DONE]"));
    Ok(())
}

#[tokio::test]
async fn completions_endpoint_returns_openai_error_for_malformed_json(
) -> Result<(), Box<dyn std::error::Error>> {
    let app = router(ServerState::new("fixture-model".to_owned()));
    let request = Request::builder()
        .method("POST")
        .uri("/v1/completions")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"model":"fixture-model","prompt":"hello""#))?;
    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_json(response.into_body()).await?;
    assert_eq!(body["error"]["type"], "invalid_request_error");
    Ok(())
}

#[tokio::test]
async fn completions_endpoint_returns_openai_error_for_missing_json_content_type(
) -> Result<(), Box<dyn std::error::Error>> {
    let app = router(ServerState::new("fixture-model".to_owned()));
    let request = Request::builder()
        .method("POST")
        .uri("/v1/completions")
        .body(Body::from(r#"{"model":"fixture-model","prompt":"hello"}"#))?;
    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_json(response.into_body()).await?;
    assert_eq!(body["error"]["type"], "invalid_request_error");
    Ok(())
}

#[tokio::test]
async fn completions_endpoint_returns_429_when_inference_is_busy(
) -> Result<(), Box<dyn std::error::Error>> {
    let model_path = write_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let state = ServerState::with_engine("fixture-model".to_owned(), engine);
    let _held_permit = state
        .try_acquire_inference_permit()
        .ok_or("expected first inference permit")?;
    let app = router(state);
    let request = Request::builder()
        .method("POST")
        .uri("/v1/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"model":"fixture-model","prompt":"hello","max_tokens":1}"#,
        ))?;
    let response = app.oneshot(request).await?;
    remove_fixture_model(&model_path)?;

    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    let body = to_json(response.into_body()).await?;
    assert_eq!(body["error"]["type"], "rate_limit_error");
    Ok(())
}

async fn to_json(body: Body) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str(&to_text(body).await?)?)
}

async fn to_text(body: Body) -> Result<String, Box<dyn std::error::Error>> {
    let bytes = to_bytes(body, usize::MAX).await?;
    Ok(String::from_utf8(bytes.to_vec())?)
}

fn write_fixture_model() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    write_fixture_model_bytes(ferrite_fixtures::scalar_llama_f32_gguf_fixture())
}

fn write_chat_fixture_model() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    write_fixture_model_bytes(ferrite_fixtures::scalar_llama_chat_f32_gguf_fixture())
}

fn write_fixture_model_bytes(
    bytes: Vec<u8>,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "ferrite-server-fixture-{}-{}.gguf",
        std::process::id(),
        FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&path, bytes)?;
    Ok(path)
}

fn remove_fixture_model(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}
