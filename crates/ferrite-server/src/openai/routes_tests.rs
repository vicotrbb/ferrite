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

async fn to_json(body: Body) -> Result<Value, Box<dyn std::error::Error>> {
    let bytes = to_bytes(body, usize::MAX).await?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn write_fixture_model() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "ferrite-server-fixture-{}-{}.gguf",
        std::process::id(),
        FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&path, ferrite_fixtures::scalar_llama_f32_gguf_fixture())?;
    Ok(path)
}

fn remove_fixture_model(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}
