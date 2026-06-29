use super::response_shape_assertions::{
    assert_null_system_fingerprint, assert_usage_has_detail_counters,
};
use super::routes::router;
use super::test_support::{
    remove_fixture_model, to_json, write_chat_fixture_model, write_fixture_model,
};
use crate::{runtime::InferenceEngine, state::ServerState};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

#[tokio::test]
async fn completions_endpoint_returns_openai_choice_shape() -> Result<(), Box<dyn std::error::Error>>
{
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

    let status = response.status();
    let body = to_json(response.into_body()).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_null_system_fingerprint(&body)?;

    let choice = body["choices"][0]
        .as_object()
        .ok_or("expected completion choice object")?;
    assert!(choice.contains_key("logprobs"), "{body}");
    assert!(choice["logprobs"].is_null(), "{body}");
    assert_eq!(choice["finish_reason"], "length");
    assert_eq!(choice["text"], "winner");
    assert_usage_has_detail_counters(&body["usage"])?;
    Ok(())
}

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
    assert_null_system_fingerprint(&body)?;

    let choice = body["choices"][0]
        .as_object()
        .ok_or("expected choice object")?;
    assert!(choice.contains_key("logprobs"), "{body}");
    assert!(choice["logprobs"].is_null(), "{body}");
    assert_eq!(choice["finish_reason"], "length");

    let message = choice["message"]
        .as_object()
        .ok_or("expected message object")?;
    assert!(message.contains_key("refusal"), "{body}");
    assert!(message["refusal"].is_null(), "{body}");
    assert!(message.contains_key("annotations"), "{body}");
    assert_eq!(message["annotations"], serde_json::json!([]));
    assert_eq!(message["role"], "assistant");
    assert_eq!(message["content"], "winner");
    assert_usage_has_detail_counters(&body["usage"])?;
    Ok(())
}
