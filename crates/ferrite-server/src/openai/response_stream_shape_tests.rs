use super::response_shape_assertions::{
    assert_choice_has_null_logprobs, assert_null_system_fingerprint, json_sse_events,
};
use super::routes::router;
use super::test_support::{
    remove_fixture_model, to_text, write_chat_fixture_model, write_fixture_model,
};
use crate::{runtime::InferenceEngine, state::ServerState};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

#[tokio::test]
async fn completions_stream_endpoint_returns_openai_choice_shape(
) -> Result<(), Box<dyn std::error::Error>> {
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

    let status = response.status();
    let body = to_text(response.into_body()).await?;
    assert_eq!(status, StatusCode::OK, "{body}");

    let events = json_sse_events(&body)?;
    let token_event = events
        .iter()
        .find(|event| event["choices"][0]["text"] == "winner")
        .ok_or("expected token event")?;
    assert_null_system_fingerprint(token_event)?;
    assert_choice_has_null_logprobs(token_event)?;

    let length_event = events
        .iter()
        .find(|event| event["choices"][0]["finish_reason"] == "length")
        .ok_or("expected length event")?;
    assert_null_system_fingerprint(length_event)?;
    assert_choice_has_null_logprobs(length_event)?;
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
    assert_null_system_fingerprint(role_event)?;
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
    assert_null_system_fingerprint(token_event)?;
    assert_choice_has_null_logprobs(token_event)?;

    let length_event = events
        .iter()
        .find(|event| event["choices"][0]["finish_reason"] == "length")
        .ok_or("expected length event")?;
    assert_null_system_fingerprint(length_event)?;
    assert_choice_has_null_logprobs(length_event)?;
    Ok(())
}
