use super::routes::router;
use super::test_support::{remove_fixture_model, to_json, write_fixture_model};
use crate::{runtime::InferenceEngine, state::ServerState};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tokio::time::{sleep, Duration};
use tower::ServiceExt;

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
