use super::routes::router;
use super::test_support::to_json;
use crate::state::ServerState;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

#[tokio::test]
async fn chat_endpoint_reports_unloaded_model_before_busy_queue(
) -> Result<(), Box<dyn std::error::Error>> {
    let state = ServerState::new("fixture-model".to_owned());
    let _held_permit = state
        .try_acquire_inference_permit()
        .ok_or("expected held inference permit")?;
    let app = router(state);
    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"model":"fixture-model","messages":[{"role":"user","content":"hello"}],"max_completion_tokens":1}"#,
        ))?;
    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = to_json(response.into_body()).await?;
    assert_eq!(body["error"]["type"], "server_error");
    assert!(body["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("no model is loaded"));
    Ok(())
}

#[tokio::test]
async fn completion_endpoint_reports_unloaded_model_before_busy_queue(
) -> Result<(), Box<dyn std::error::Error>> {
    let state = ServerState::new("fixture-model".to_owned());
    let _held_permit = state
        .try_acquire_inference_permit()
        .ok_or("expected held inference permit")?;
    let app = router(state);
    let request = Request::builder()
        .method("POST")
        .uri("/v1/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"model":"fixture-model","prompt":"hello","max_tokens":1}"#,
        ))?;
    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = to_json(response.into_body()).await?;
    assert_eq!(body["error"]["type"], "server_error");
    assert!(body["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("no model is loaded"));
    Ok(())
}

#[tokio::test]
async fn streaming_completion_prompt_validation_runs_before_engine_availability(
) -> Result<(), Box<dyn std::error::Error>> {
    let app = router(ServerState::new("fixture-model".to_owned()));
    let request = Request::builder()
        .method("POST")
        .uri("/v1/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"model":"fixture-model","prompt":["hello","again"],"stream":true,"max_tokens":1}"#,
        ))?;
    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_json(response.into_body()).await?;
    assert_eq!(body["error"]["type"], "invalid_request_error");
    assert!(body["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("exactly one text prompt"));
    Ok(())
}
