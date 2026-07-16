use super::routes::router;
use crate::{runtime::InferenceEngine, state::ServerState};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use std::sync::atomic::{AtomicU64, Ordering};
use tower::ServiceExt;

static FIXTURE_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(super) struct StreamResponseText {
    pub status: StatusCode,
    pub body: String,
}

pub(super) async fn post_loaded_completion_stream(
    payload: &str,
) -> Result<StreamResponseText, Box<dyn std::error::Error>> {
    let model_path = write_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let app = router(ServerState::with_engine("fixture-model".to_owned(), engine));
    let response = app
        .oneshot(post_request("/v1/completions", payload)?)
        .await?;
    remove_fixture_model(&model_path)?;
    response_text(response).await
}

pub(super) async fn post_loaded_chat_stream(
    payload: &str,
) -> Result<StreamResponseText, Box<dyn std::error::Error>> {
    let model_path = write_chat_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let app = router(ServerState::with_engine("fixture-model".to_owned(), engine));
    let response = app
        .oneshot(post_request("/v1/chat/completions", payload)?)
        .await?;
    remove_fixture_model(&model_path)?;
    response_text(response).await
}

pub(super) async fn post_unloaded_completion(
    payload: &str,
) -> Result<StreamResponseText, Box<dyn std::error::Error>> {
    let app = router(ServerState::new("fixture-model".to_owned()));
    let response = app
        .oneshot(post_request("/v1/completions", payload)?)
        .await?;
    response_text(response).await
}

pub(super) async fn post_unloaded_chat(
    payload: &str,
) -> Result<StreamResponseText, Box<dyn std::error::Error>> {
    let app = router(ServerState::new("fixture-model".to_owned()));
    let response = app
        .oneshot(post_request("/v1/chat/completions", payload)?)
        .await?;
    response_text(response).await
}

fn post_request(uri: &str, payload: &str) -> Result<Request<Body>, Box<dyn std::error::Error>> {
    Ok(Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(payload.to_owned()))?)
}

async fn response_text(
    response: axum::response::Response,
) -> Result<StreamResponseText, Box<dyn std::error::Error>> {
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await?;
    let body = String::from_utf8(bytes.to_vec())?;
    Ok(StreamResponseText { status, body })
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
        "ferrite-server-stream-options-fixture-{}-{}.gguf",
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
