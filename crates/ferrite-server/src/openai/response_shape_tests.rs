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

#[tokio::test]
async fn chat_stream_endpoint_returns_openai_choice_shape() -> Result<(), Box<dyn std::error::Error>>
{
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

    let status = response.status();
    let body = to_text(response.into_body()).await?;
    assert_eq!(status, StatusCode::OK, "{body}");

    let events = json_sse_events(&body)?;
    let role_event = events.first().ok_or("expected role event")?;
    let role_choice = role_event["choices"][0]
        .as_object()
        .ok_or("expected role choice object")?;
    assert_eq!(role_choice["delta"]["role"], "assistant", "{role_event}");
    assert_eq!(role_choice["delta"]["content"], "", "{role_event}");
    assert!(role_choice["finish_reason"].is_null(), "{role_event}");
    assert_choice_has_null_logprobs(role_event)?;

    let token_event = events
        .iter()
        .find(|event| event["choices"][0]["delta"]["content"] == "winner")
        .ok_or("expected token event")?;
    assert_choice_has_null_logprobs(token_event)?;

    let stop_event = events
        .iter()
        .find(|event| event["choices"][0]["finish_reason"] == "stop")
        .ok_or("expected stop event")?;
    assert_choice_has_null_logprobs(stop_event)?;
    Ok(())
}

async fn to_json(body: Body) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str(&to_text(body).await?)?)
}

async fn to_text(body: Body) -> Result<String, Box<dyn std::error::Error>> {
    let bytes = to_bytes(body, usize::MAX).await?;
    Ok(String::from_utf8(bytes.to_vec())?)
}

fn json_sse_events(body: &str) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    body.lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .filter(|data| *data != "[DONE]")
        .map(|data| Ok(serde_json::from_str(data)?))
        .collect()
}

fn assert_choice_has_null_logprobs(event: &Value) -> Result<(), Box<dyn std::error::Error>> {
    let choice = event["choices"][0]
        .as_object()
        .ok_or("expected streamed choice object")?;
    assert!(choice.contains_key("logprobs"), "{event}");
    assert!(choice["logprobs"].is_null(), "{event}");
    Ok(())
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
