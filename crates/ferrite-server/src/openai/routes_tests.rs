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
async fn completions_endpoint_generates_with_loaded_fixture_model()
-> Result<(), Box<dyn std::error::Error>> {
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
async fn completions_endpoint_accepts_array_of_string_prompts()
-> Result<(), Box<dyn std::error::Error>> {
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
async fn chat_endpoint_reports_cached_tokens_when_experimental_prefix_cache_is_enabled()
-> Result<(), Box<dyn std::error::Error>> {
    let model_path = write_chat_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let state = ServerState::with_engine("fixture-model".to_owned(), engine)
        .with_prefix_cache_enabled(true)
        .with_batched_decode(2)?;
    let app = router(state);
    let request_body = r#"{"model":"fixture-model","messages":[{"role":"user","content":"hello"}],"prompt_cache_key":"tenant-a:thread-1","max_completion_tokens":1}"#;

    let first_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("content-type", "application/json")
                .body(Body::from(request_body))?,
        )
        .await?;
    let second_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("content-type", "application/json")
                .body(Body::from(request_body))?,
        )
        .await?;
    remove_fixture_model(&model_path)?;

    let first_status = first_response.status();
    let first_body = to_json(first_response.into_body()).await?;
    assert_eq!(first_status, StatusCode::OK, "{first_body}");
    assert_eq!(
        first_body["usage"]["prompt_tokens_details"]["cached_tokens"],
        0
    );

    let second_status = second_response.status();
    let second_body = to_json(second_response.into_body()).await?;
    assert_eq!(second_status, StatusCode::OK, "{second_body}");
    assert_eq!(second_body["choices"][0]["message"]["content"], "winner");
    assert_eq!(second_body["usage"]["prompt_tokens"], 4);
    assert_eq!(
        second_body["usage"]["prompt_tokens_details"]["cached_tokens"],
        4
    );
    Ok(())
}

#[tokio::test]
async fn completions_endpoint_reports_cached_tokens_when_experimental_prefix_cache_is_enabled()
-> Result<(), Box<dyn std::error::Error>> {
    let model_path = write_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let state = ServerState::with_engine("fixture-model".to_owned(), engine)
        .with_prefix_cache_enabled(true)
        .with_batched_decode(2)?;
    let app = router(state);
    let request_body = r#"{"model":"fixture-model","prompt":"hello","prompt_cache_key":"tenant-a:completion-1","max_tokens":1}"#;

    let first_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/completions")
                .header("content-type", "application/json")
                .body(Body::from(request_body))?,
        )
        .await?;
    let second_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/completions")
                .header("content-type", "application/json")
                .body(Body::from(request_body))?,
        )
        .await?;
    remove_fixture_model(&model_path)?;

    let first_status = first_response.status();
    let first_body = to_json(first_response.into_body()).await?;
    assert_eq!(first_status, StatusCode::OK, "{first_body}");
    assert_eq!(
        first_body["usage"]["prompt_tokens_details"]["cached_tokens"],
        0
    );

    let second_status = second_response.status();
    let second_body = to_json(second_response.into_body()).await?;
    assert_eq!(second_status, StatusCode::OK, "{second_body}");
    assert_eq!(second_body["choices"][0]["text"], "winner");
    assert_eq!(second_body["usage"]["prompt_tokens"], 1);
    assert_eq!(
        second_body["usage"]["prompt_tokens_details"]["cached_tokens"],
        1
    );
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_accepts_assistant_refusal_content_parts()
-> Result<(), Box<dyn std::error::Error>> {
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
async fn chat_endpoint_accepts_null_assistant_refusal_message_metadata()
-> Result<(), Box<dyn std::error::Error>> {
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
async fn chat_endpoint_accepts_null_assistant_audio_message_metadata()
-> Result<(), Box<dyn std::error::Error>> {
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
async fn chat_endpoint_accepts_deprecated_function_message_role()
-> Result<(), Box<dyn std::error::Error>> {
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
