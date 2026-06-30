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
async fn completions_endpoint_streams_echo_prompt_when_requested(
) -> Result<(), Box<dyn std::error::Error>> {
    let model_path = write_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let app = router(ServerState::with_engine("fixture-model".to_owned(), engine));
    let request = Request::builder()
        .method("POST")
        .uri("/v1/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"model":"fixture-model","prompt":"hello","max_tokens":1,"stream":true,"echo":true}"#,
        ))?;
    let response = app.oneshot(request).await?;
    remove_fixture_model(&model_path)?;

    let status = response.status();
    let body = to_text(response.into_body()).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    let prompt_position = body
        .find("\"text\":\"hello\"")
        .ok_or("missing echo prompt")?;
    let generated_position = body
        .find("\"text\":\"winner\"")
        .ok_or("missing generated token")?;
    assert!(prompt_position < generated_position, "{body}");
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
async fn completion_stream_helper_emits_tokens_from_generation_callback(
) -> Result<(), Box<dyn std::error::Error>> {
    let model_path = write_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    remove_fixture_model(&model_path)?;
    let state = ServerState::with_engine("fixture-model".to_owned(), engine);
    let permit = state
        .try_acquire_inference_permit()
        .ok_or("expected inference permit")?;

    let response = super::stream_generation::completion_stream_response(
        state.engine().ok_or("expected inference engine")?,
        "fixture-model".to_owned(),
        "hello".to_owned(),
        1,
        super::stream_generation::CompletionStreamOptions::new(Vec::new(), false, false),
        permit,
    );
    let body = to_text(response.into_body()).await?;

    assert!(body.contains("\"text\":\"winner\""));
    assert!(body.contains("data: [DONE]"));
    Ok(())
}
