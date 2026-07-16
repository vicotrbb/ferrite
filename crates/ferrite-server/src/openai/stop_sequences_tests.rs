use super::{
    routes::router,
    test_support::{
        remove_fixture_model, to_json, to_text, write_chat_fixture_model,
        write_chat_fixture_model_with_eos_token_id, write_fixture_model,
        write_fixture_model_with_eos_token_id,
    },
};
use crate::{runtime::InferenceEngine, state::ServerState};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn completions_endpoint_applies_string_stop_sequence()
-> Result<(), Box<dyn std::error::Error>> {
    let body = post_completion(
        r#"{"model":"fixture-model","prompt":"hello","max_tokens":1,"stop":"ner"}"#,
    )
    .await?;

    assert_eq!(body["choices"][0]["text"], "win");
    assert_eq!(body["usage"]["completion_tokens"], 1);
    Ok(())
}

#[tokio::test]
async fn completions_endpoint_stops_generation_when_stop_sequence_matches()
-> Result<(), Box<dyn std::error::Error>> {
    let body = post_completion(
        r#"{"model":"fixture-model","prompt":"hello","max_tokens":3,"stop":"ner"}"#,
    )
    .await?;

    assert_eq!(body["choices"][0]["text"], "win");
    assert_eq!(body["usage"]["completion_tokens"], 1);
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_applies_string_stop_sequence() -> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat(
        r#"{"model":"fixture-model","messages":[{"role":"user","content":"hello"}],"max_completion_tokens":1,"stop":"ner"}"#,
    )
    .await?;

    assert_eq!(body["choices"][0]["message"]["content"], "win");
    assert_eq!(body["usage"]["completion_tokens"], 1);
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_stops_generation_when_stop_sequence_matches()
-> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat(
        r#"{"model":"fixture-model","messages":[{"role":"user","content":"hello"}],"max_completion_tokens":3,"stop":"ner"}"#,
    )
    .await?;

    assert_eq!(body["choices"][0]["message"]["content"], "win");
    assert_eq!(body["usage"]["completion_tokens"], 1);
    Ok(())
}

#[tokio::test]
async fn completions_stream_endpoint_applies_string_stop_sequence()
-> Result<(), Box<dyn std::error::Error>> {
    let body = post_completion_stream(
        r#"{"model":"fixture-model","prompt":"hello","max_tokens":1,"stream":true,"stop":"ner"}"#,
    )
    .await?;

    assert!(body.contains("\"text\":\"win\""), "{body}");
    assert!(!body.contains("\"text\":\"winner\""), "{body}");
    assert!(!body.contains("\"token_ids\""), "{body}");
    assert!(body.contains("\"finish_reason\":\"stop\""), "{body}");
    assert!(body.contains("data: [DONE]"), "{body}");
    Ok(())
}

#[tokio::test]
async fn completions_stream_endpoint_flushes_chunks_when_stop_sequence_does_not_match()
-> Result<(), Box<dyn std::error::Error>> {
    let body = post_completion_stream(
        r#"{"model":"fixture-model","prompt":"hello","max_tokens":2,"stream":true,"stop":"zzz"}"#,
    )
    .await?;

    let events = sse_json_events(&body)?;
    let text_chunks = completion_text_chunks(&events);
    assert_eq!(text_chunks.len(), 2, "{body}");
    assert_eq!(text_chunks[0], "winner");
    assert!(!body.contains("\"token_ids\""), "{body}");
    assert!(
        events
            .iter()
            .any(|event| event["choices"][0]["finish_reason"] == "length"),
        "{body}"
    );
    assert!(body.contains("data: [DONE]"), "{body}");
    Ok(())
}

#[tokio::test]
async fn chat_stream_endpoint_applies_string_stop_sequence()
-> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat_stream(
        r#"{"model":"fixture-model","messages":[{"role":"user","content":"hello"}],"max_completion_tokens":1,"stream":true,"stop":"ner"}"#,
    )
    .await?;

    assert!(body.contains("\"delta\":{\"content\":\"win\"}"), "{body}");
    assert!(
        !body.contains("\"delta\":{\"content\":\"winner\"}"),
        "{body}"
    );
    assert!(!body.contains("\"token_ids\""), "{body}");
    assert!(body.contains("\"finish_reason\":\"stop\""), "{body}");
    assert!(body.contains("data: [DONE]"), "{body}");
    Ok(())
}

#[tokio::test]
async fn chat_stream_endpoint_flushes_chunks_when_stop_sequence_does_not_match()
-> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat_stream(
        r#"{"model":"fixture-model","messages":[{"role":"user","content":"hello"}],"max_completion_tokens":2,"stream":true,"stop":"zzz"}"#,
    )
    .await?;

    let events = sse_json_events(&body)?;
    let content_chunks = chat_content_chunks(&events);
    assert_eq!(content_chunks.len(), 2, "{body}");
    assert_eq!(content_chunks[0], "winner");
    assert!(
        events
            .iter()
            .any(|event| event["choices"][0]["finish_reason"] == "length"),
        "{body}"
    );
    assert!(body.contains("data: [DONE]"), "{body}");
    Ok(())
}

#[tokio::test]
async fn completions_endpoint_suppresses_visible_eos_text() -> Result<(), Box<dyn std::error::Error>>
{
    let body = post_completion_with_eos_token_id(
        r#"{"model":"fixture-model","prompt":"hello","max_tokens":3}"#,
        2,
    )
    .await?;

    assert_eq!(body["choices"][0]["text"], "");
    assert_eq!(body["choices"][0]["finish_reason"], "stop");
    assert_eq!(body["usage"]["completion_tokens"], 1);
    Ok(())
}

#[tokio::test]
async fn completions_stream_endpoint_suppresses_visible_eos_text()
-> Result<(), Box<dyn std::error::Error>> {
    let body = post_completion_stream_with_eos_token_id(
        r#"{"model":"fixture-model","prompt":"hello","max_tokens":3,"stream":true,"stream_options":{"include_usage":true}}"#,
        2,
    )
    .await?;

    let events = sse_json_events(&body)?;
    assert_eq!(
        completion_text_chunks(&events),
        Vec::<&str>::new(),
        "{body}"
    );
    assert!(
        events
            .iter()
            .any(|event| event["choices"][0]["finish_reason"] == "stop"),
        "{body}"
    );
    assert!(body.contains("\"completion_tokens\":1"), "{body}");
    assert!(body.contains("data: [DONE]"), "{body}");
    Ok(())
}

#[tokio::test]
async fn chat_stream_endpoint_reports_eos_finish_source_in_usage()
-> Result<(), Box<dyn std::error::Error>> {
    let body = post_chat_stream_with_eos_token_id(
        r#"{"model":"fixture-model","messages":[{"role":"user","content":"hello"}],"max_completion_tokens":3,"stream":true,"stream_options":{"include_usage":true}}"#,
        2,
    )
    .await?;

    let events = sse_json_events(&body)?;
    assert_eq!(chat_content_chunks(&events), Vec::<&str>::new(), "{body}");
    assert!(
        events
            .iter()
            .any(|event| event["choices"][0]["finish_reason"] == "stop"),
        "{body}"
    );
    assert!(
        events.iter().any(|event| {
            event["usage"]["completion_tokens_details"]["ferrite_finish_source"] == "eos"
        }),
        "{body}"
    );
    assert!(body.contains("data: [DONE]"), "{body}");
    Ok(())
}

async fn post_completion(payload: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let model_path = write_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let app = router(ServerState::with_engine("fixture-model".to_owned(), engine));
    let request = Request::builder()
        .method("POST")
        .uri("/v1/completions")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_owned()))?;
    let response = app.oneshot(request).await?;
    remove_fixture_model(&model_path)?;

    let status = response.status();
    let body = to_json(response.into_body()).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    Ok(body)
}

async fn post_completion_with_eos_token_id(
    payload: &str,
    eos_token_id: u64,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let model_path = write_fixture_model_with_eos_token_id(eos_token_id)?;
    let engine = InferenceEngine::load(&model_path)?;
    let app = router(ServerState::with_engine("fixture-model".to_owned(), engine));
    let request = Request::builder()
        .method("POST")
        .uri("/v1/completions")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_owned()))?;
    let response = app.oneshot(request).await?;
    remove_fixture_model(&model_path)?;

    let status = response.status();
    let body = to_json(response.into_body()).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    Ok(body)
}

async fn post_chat(payload: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let model_path = write_chat_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let app = router(ServerState::with_engine("fixture-model".to_owned(), engine));
    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_owned()))?;
    let response = app.oneshot(request).await?;
    remove_fixture_model(&model_path)?;

    let status = response.status();
    let body = to_json(response.into_body()).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    Ok(body)
}

async fn post_completion_stream(payload: &str) -> Result<String, Box<dyn std::error::Error>> {
    post_stream(payload, "/v1/completions", write_fixture_model).await
}

async fn post_completion_stream_with_eos_token_id(
    payload: &str,
    eos_token_id: u64,
) -> Result<String, Box<dyn std::error::Error>> {
    let model_path = write_fixture_model_with_eos_token_id(eos_token_id)?;
    let engine = InferenceEngine::load(&model_path)?;
    let app = router(ServerState::with_engine("fixture-model".to_owned(), engine));
    let request = Request::builder()
        .method("POST")
        .uri("/v1/completions")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_owned()))?;
    let response = app.oneshot(request).await?;
    remove_fixture_model(&model_path)?;

    let status = response.status();
    let body = to_text(response.into_body()).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    Ok(body)
}

async fn post_chat_stream(payload: &str) -> Result<String, Box<dyn std::error::Error>> {
    post_stream(payload, "/v1/chat/completions", write_chat_fixture_model).await
}

async fn post_chat_stream_with_eos_token_id(
    payload: &str,
    eos_token_id: u64,
) -> Result<String, Box<dyn std::error::Error>> {
    let model_path = write_chat_fixture_model_with_eos_token_id(eos_token_id)?;
    let engine = InferenceEngine::load(&model_path)?;
    let app = router(ServerState::with_engine("fixture-model".to_owned(), engine));
    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_owned()))?;
    let response = app.oneshot(request).await?;
    remove_fixture_model(&model_path)?;

    let status = response.status();
    let body = to_text(response.into_body()).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    Ok(body)
}

async fn post_stream(
    payload: &str,
    uri: &str,
    write_model: fn() -> Result<std::path::PathBuf, Box<dyn std::error::Error>>,
) -> Result<String, Box<dyn std::error::Error>> {
    let model_path = write_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let app = router(ServerState::with_engine("fixture-model".to_owned(), engine));
    let request = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(payload.to_owned()))?;
    let response = app.oneshot(request).await?;
    remove_fixture_model(&model_path)?;

    let status = response.status();
    let body = to_text(response.into_body()).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    Ok(body)
}

fn sse_json_events(response: &str) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    response
        .lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .filter(|payload| *payload != "[DONE]")
        .map(|payload| Ok(serde_json::from_str(payload)?))
        .collect()
}

fn completion_text_chunks(events: &[Value]) -> Vec<&str> {
    events
        .iter()
        .filter_map(|event| {
            let choice = &event["choices"][0];
            choice["finish_reason"]
                .is_null()
                .then(|| choice["text"].as_str())?
        })
        .collect()
}

fn chat_content_chunks(events: &[Value]) -> Vec<&str> {
    events
        .iter()
        .filter_map(|event| {
            let choice = &event["choices"][0];
            (choice["finish_reason"].is_null() && choice["delta"]["role"].is_null())
                .then(|| choice["delta"]["content"].as_str())?
        })
        .collect()
}
