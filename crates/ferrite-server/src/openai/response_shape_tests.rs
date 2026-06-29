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
async fn chat_endpoint_returns_openai_message_shape() -> Result<(), Box<dyn std::error::Error>> {
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

    let status = response.status();
    let body = to_json(response.into_body()).await?;
    assert_eq!(status, StatusCode::OK, "{body}");

    let choice = body["choices"][0]
        .as_object()
        .ok_or("expected choice object")?;
    assert!(choice.contains_key("logprobs"), "{body}");
    assert!(choice["logprobs"].is_null(), "{body}");

    let message = choice["message"]
        .as_object()
        .ok_or("expected message object")?;
    assert!(message.contains_key("refusal"), "{body}");
    assert!(message["refusal"].is_null(), "{body}");
    assert!(message.contains_key("annotations"), "{body}");
    assert_eq!(message["annotations"], serde_json::json!([]));
    assert_eq!(message["role"], "assistant");
    assert_eq!(message["content"], "winner");
    Ok(())
}

async fn to_json(body: Body) -> Result<Value, Box<dyn std::error::Error>> {
    let bytes = to_bytes(body, usize::MAX).await?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn write_chat_fixture_model() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "ferrite-server-response-shape-fixture-{}-{}.gguf",
        std::process::id(),
        FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(
        &path,
        ferrite_fixtures::scalar_llama_chat_f32_gguf_fixture(),
    )?;
    Ok(path)
}

fn remove_fixture_model(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}
