use super::routes::router;
use super::test_support::{
    remove_fixture_model, to_text, write_chat_fixture_model, write_fixture_model,
};
use crate::{runtime::InferenceEngine, state::ServerState};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use std::time::{Duration, Instant};
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
    assert!(body.contains("\"token_ids\":["));
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
    let events = body
        .lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .filter(|line| *line != "[DONE]")
        .map(serde_json::from_str::<serde_json::Value>)
        .collect::<Result<Vec<_>, _>>()?;
    let echo_event = events
        .iter()
        .find(|event| event["choices"][0]["text"] == "hello")
        .ok_or("missing echo event")?;
    assert!(echo_event["choices"][0].get("token_ids").is_none());
    let generated_event = events
        .iter()
        .find(|event| event["choices"][0]["text"] == "winner")
        .ok_or("missing generated event")?;
    assert!(generated_event["choices"][0]["token_ids"].is_array());
    assert!(body.contains("data: [DONE]"));
    Ok(())
}

#[tokio::test]
async fn completions_endpoint_stream_reports_cached_tokens_when_experimental_prefix_cache_is_enabled(
) -> Result<(), Box<dyn std::error::Error>> {
    let model_path = write_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let state = ServerState::with_engine("fixture-model".to_owned(), engine)
        .with_prefix_cache_enabled(true)
        .with_batched_decode(2)?;
    let app = router(state);
    let request_body = r#"{"model":"fixture-model","prompt":"hello","prompt_cache_key":"tenant-a:completion-stream-1","max_tokens":1,"stream":true,"stream_options":{"include_usage":true}}"#;

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
    let first_status = first_response.status();
    let first_body = to_text(first_response.into_body()).await?;
    assert_eq!(first_status, StatusCode::OK, "{first_body}");
    assert_eq!(stream_usage_cached_tokens(&first_body)?, 0);

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

    let second_status = second_response.status();
    let second_body = to_text(second_response.into_body()).await?;
    assert_eq!(second_status, StatusCode::OK, "{second_body}");
    assert!(second_body.contains("\"text\":\"winner\""), "{second_body}");
    assert_eq!(stream_usage_cached_tokens(&second_body)?, 1);
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
    assert!(body.contains("\"token_ids\":["));
    assert!(body.contains("data: [DONE]"));
    Ok(())
}

#[tokio::test]
async fn sampled_completion_stream_is_seeded_and_bypasses_greedy_batch_admission(
) -> Result<(), Box<dyn std::error::Error>> {
    let model_path = write_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let state =
        ServerState::with_engine("fixture-model".to_owned(), engine).with_batched_decode(1)?;
    let held_batch_permit = state
        .try_acquire_batch_admission_permit()
        .ok_or("expected batch admission permit")?;
    let app = router(state);

    let first = sampled_completion_stream_body(app.clone()).await?;
    let second = sampled_completion_stream_body(app).await?;
    drop(held_batch_permit);
    remove_fixture_model(&model_path)?;

    assert_eq!(
        completion_stream_signature(&first)?,
        completion_stream_signature(&second)?
    );
    assert!(first.contains("data: [DONE]"));
    Ok(())
}

#[tokio::test]
async fn batched_completion_streams_match_default_path_under_parallel_load(
) -> Result<(), Box<dyn std::error::Error>> {
    let model_path = write_fixture_model()?;
    let default_engine = InferenceEngine::load(&model_path)?;
    let default_app = router(ServerState::with_engine(
        "fixture-model".to_owned(),
        default_engine,
    ));
    let default_body = completion_stream_body(default_app, 4).await?;

    let batched_engine = InferenceEngine::load(&model_path)?;
    let batched_state = ServerState::with_engine("fixture-model".to_owned(), batched_engine)
        .with_batched_decode(2)?;
    let batched_app = router(batched_state);
    let (first, second) = tokio::join!(
        completion_stream_body(batched_app.clone(), 4),
        completion_stream_body(batched_app, 4),
    );
    remove_fixture_model(&model_path)?;

    let expected = completion_stream_signature(&default_body)?;
    assert_eq!(completion_stream_signature(&first?)?, expected);
    assert_eq!(completion_stream_signature(&second?)?, expected);
    Ok(())
}

#[tokio::test]
async fn batched_stream_releases_admission_permit_when_response_body_is_dropped(
) -> Result<(), Box<dyn std::error::Error>> {
    let model_path = write_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let state =
        ServerState::with_engine("fixture-model".to_owned(), engine).with_batched_decode(1)?;
    let held_queue_permit = state
        .try_acquire_batch_admission_permit()
        .ok_or("expected spare batch admission permit")?;
    let app = router(state.clone());
    let response = app.oneshot(completion_stream_request(128)?).await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(state.try_acquire_batch_admission_permit().is_none());

    drop(response);
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if state.try_acquire_batch_admission_permit().is_some() {
            break;
        }
        if Instant::now() >= deadline {
            remove_fixture_model(&model_path)?;
            return Err("batched stream kept its admission permit after body drop".into());
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    drop(held_queue_permit);
    remove_fixture_model(&model_path)?;
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
        crate::runtime::GenerationCacheOptions::default(),
        permit,
    );
    let body = to_text(response.into_body()).await?;

    assert!(body.contains("\"text\":\"winner\""));
    assert!(body.contains("\"token_ids\":["));
    assert!(body.contains("data: [DONE]"));
    Ok(())
}

#[tokio::test]
async fn chat_stream_releases_inference_permit_when_response_body_is_dropped(
) -> Result<(), Box<dyn std::error::Error>> {
    let model_path = write_chat_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let state = ServerState::with_engine("fixture-model".to_owned(), engine);
    let app = router(state.clone());
    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"model":"fixture-model","messages":[{"role":"user","content":"hello"}],"max_tokens":128,"stream":true}"#,
        ))?;

    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(state.try_acquire_inference_permit().is_none());

    drop(response);

    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if state.try_acquire_inference_permit().is_some() {
            break;
        }
        if Instant::now() >= deadline {
            remove_fixture_model(&model_path)?;
            return Err("streaming request kept the inference permit after body drop".into());
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    remove_fixture_model(&model_path)?;
    Ok(())
}

fn stream_usage_cached_tokens(body: &str) -> Result<u64, Box<dyn std::error::Error>> {
    for line in body.lines().filter_map(|line| line.strip_prefix("data: ")) {
        if line == "[DONE]" {
            continue;
        }
        let event: serde_json::Value = serde_json::from_str(line)?;
        if let Some(tokens) = event["usage"]["prompt_tokens_details"]["cached_tokens"].as_u64() {
            return Ok(tokens);
        }
    }
    Err(format!("missing stream usage cached_tokens: {body}").into())
}

fn completion_stream_request(
    max_tokens: usize,
) -> Result<Request<Body>, Box<dyn std::error::Error>> {
    Ok(Request::builder()
        .method("POST")
        .uri("/v1/completions")
        .header("content-type", "application/json")
        .body(Body::from(format!(
            r#"{{"model":"fixture-model","prompt":"hello","max_tokens":{max_tokens},"stream":true,"stream_options":{{"include_usage":true}}}}"#
        )))?)
}

async fn completion_stream_body(
    app: axum::Router,
    max_tokens: usize,
) -> Result<String, Box<dyn std::error::Error>> {
    let response = app.oneshot(completion_stream_request(max_tokens)?).await?;
    let status = response.status();
    let body = to_text(response.into_body()).await?;
    if status != StatusCode::OK {
        return Err(format!("completion stream returned {status}: {body}").into());
    }
    Ok(body)
}

async fn sampled_completion_stream_body(
    app: axum::Router,
) -> Result<String, Box<dyn std::error::Error>> {
    let request = Request::builder()
        .method("POST")
        .uri("/v1/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"model":"fixture-model","prompt":"hello","max_tokens":4,"stream":true,"temperature":1,"top_k":2,"seed":42,"stream_options":{"include_usage":true}}"#,
        ))?;
    let response = app.oneshot(request).await?;
    let status = response.status();
    let body = to_text(response.into_body()).await?;
    if status != StatusCode::OK {
        return Err(format!("sampled completion stream returned {status}: {body}").into());
    }
    Ok(body)
}

fn completion_stream_signature(
    body: &str,
) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
    body.lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .filter(|line| *line != "[DONE]")
        .map(|line| {
            let event: serde_json::Value = serde_json::from_str(line)?;
            Ok(serde_json::json!({
                "text": event["choices"][0]["text"],
                "finish_reason": event["choices"][0]["finish_reason"],
                "token_ids": event["choices"][0]["token_ids"],
                "usage": event["usage"],
            }))
        })
        .collect()
}
