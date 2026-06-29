use super::routes::router;
use super::test_support::{
    remove_fixture_model, to_json, to_text, write_chat_fixture_model, write_fixture_model,
};
use crate::{runtime::InferenceEngine, state::ServerState};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tokio::time::{sleep, Duration};
use tower::ServiceExt;

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
async fn completions_endpoint_accepts_array_of_string_prompts(
) -> Result<(), Box<dyn std::error::Error>> {
    let model_path = write_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let app = router(ServerState::with_engine("fixture-model".to_owned(), engine));
    let request = Request::builder()
        .method("POST")
        .uri("/v1/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"model":"fixture-model","prompt":["hello","hello"],"max_tokens":1}"#,
        ))?;
    let response = app.oneshot(request).await?;
    remove_fixture_model(&model_path)?;

    let status = response.status();
    let body = to_json(response.into_body()).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["choices"][0]["index"], 0);
    assert_eq!(body["choices"][0]["text"], "winner");
    assert_eq!(body["choices"][1]["index"], 1);
    assert_eq!(body["choices"][1]["text"], "winner");
    assert_eq!(body["usage"]["prompt_tokens"], 2);
    assert_eq!(body["usage"]["completion_tokens"], 2);
    assert_eq!(body["usage"]["total_tokens"], 4);
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
async fn chat_endpoint_accepts_text_content_parts() -> Result<(), Box<dyn std::error::Error>> {
    let model_path = write_chat_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let app = router(ServerState::with_engine("fixture-model".to_owned(), engine));
    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"model":"fixture-model","messages":[{"role":"user","content":[{"type":"text","text":"he"},{"type":"text","text":"llo"}]}],"max_completion_tokens":1}"#,
        ))?;
    let response = app.oneshot(request).await?;
    remove_fixture_model(&model_path)?;

    let status = response.status();
    let body = to_json(response.into_body()).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["choices"][0]["message"]["content"], "winner");
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_accepts_assistant_refusal_content_parts(
) -> Result<(), Box<dyn std::error::Error>> {
    let model_path = write_chat_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let app = router(ServerState::with_engine("fixture-model".to_owned(), engine));
    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"model":"fixture-model","messages":[{"role":"assistant","content":[{"type":"refusal","refusal":"hello"}]}],"max_completion_tokens":1}"#,
        ))?;
    let response = app.oneshot(request).await?;
    remove_fixture_model(&model_path)?;

    let status = response.status();
    let body = to_json(response.into_body()).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["choices"][0]["message"]["content"], "winner");
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_accepts_null_assistant_refusal_message_metadata(
) -> Result<(), Box<dyn std::error::Error>> {
    let model_path = write_chat_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let app = router(ServerState::with_engine("fixture-model".to_owned(), engine));
    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"model":"fixture-model","messages":[{"role":"assistant","content":"hello","refusal":null}],"max_completion_tokens":1}"#,
        ))?;
    let response = app.oneshot(request).await?;
    remove_fixture_model(&model_path)?;

    let status = response.status();
    let body = to_json(response.into_body()).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["choices"][0]["message"]["content"], "winner");
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_accepts_null_assistant_audio_message_metadata(
) -> Result<(), Box<dyn std::error::Error>> {
    let model_path = write_chat_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let app = router(ServerState::with_engine("fixture-model".to_owned(), engine));
    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"model":"fixture-model","messages":[{"role":"assistant","content":"hello","audio":null}],"max_completion_tokens":1}"#,
        ))?;
    let response = app.oneshot(request).await?;
    remove_fixture_model(&model_path)?;

    let status = response.status();
    let body = to_json(response.into_body()).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["choices"][0]["message"]["content"], "winner");
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_accepts_deprecated_function_message_role(
) -> Result<(), Box<dyn std::error::Error>> {
    let model_path = write_chat_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let app = router(ServerState::with_engine("fixture-model".to_owned(), engine));
    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"model":"fixture-model","messages":[{"role":"function","name":"lookup","content":"hello"}],"max_completion_tokens":1}"#,
        ))?;
    let response = app.oneshot(request).await?;
    remove_fixture_model(&model_path)?;

    let status = response.status();
    let body = to_json(response.into_body()).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["choices"][0]["message"]["content"], "winner");
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

    let response = super::generation::completion_stream_response(
        state.engine().ok_or("expected inference engine")?,
        "fixture-model".to_owned(),
        "hello".to_owned(),
        1,
        super::generation::CompletionStreamOptions::new(Vec::new(), false),
        permit,
    );
    let body = to_text(response.into_body()).await?;

    assert!(body.contains("\"text\":\"winner\""));
    assert!(body.contains("data: [DONE]"));
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

#[tokio::test]
async fn completions_endpoint_waits_for_busy_inference_within_configured_timeout(
) -> Result<(), Box<dyn std::error::Error>> {
    let model_path = write_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let state = ServerState::with_engine("fixture-model".to_owned(), engine)
        .with_inference_wait_timeout(Duration::from_millis(250));
    let held_permit = state
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

    let response_task = tokio::spawn(async move {
        app.oneshot(request)
            .await
            .map_err(|error| error.to_string())
    });
    sleep(Duration::from_millis(20)).await;
    drop(held_permit);
    let response = response_task.await??;
    remove_fixture_model(&model_path)?;

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_json(response.into_body()).await?;
    assert_eq!(body["choices"][0]["text"], "winner");
    Ok(())
}
