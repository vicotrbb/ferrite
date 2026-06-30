use super::routes::router;
use super::test_support::to_json;
use crate::state::ServerState;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

#[tokio::test]
async fn chat_endpoint_returns_openai_error_when_model_is_not_loaded(
) -> Result<(), Box<dyn std::error::Error>> {
    let app = router(ServerState::new("test-model".to_owned()));
    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"model":"test-model","messages":[{"role":"user","content":"Hello"}]}"#,
        ))?;
    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = to_json(response.into_body()).await?;
    assert_eq!(body["error"]["type"], "server_error");
    Ok(())
}

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
async fn completions_endpoint_returns_model_not_found_for_unknown_model(
) -> Result<(), Box<dyn std::error::Error>> {
    let app = router(ServerState::new("fixture-model".to_owned()));
    let request = Request::builder()
        .method("POST")
        .uri("/v1/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"model":"other-model","prompt":"hello","max_tokens":1}"#,
        ))?;
    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = to_json(response.into_body()).await?;
    assert_eq!(body["error"]["type"], "invalid_request_error");
    assert_eq!(body["error"]["code"], "model_not_found");
    assert_eq!(body["error"]["param"], "model", "{body}");
    assert!(body["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("other-model"));
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_returns_model_not_found_for_unknown_model(
) -> Result<(), Box<dyn std::error::Error>> {
    let app = router(ServerState::new("fixture-model".to_owned()));
    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"model":"other-model","messages":[{"role":"user","content":"hello"}],"max_tokens":1}"#,
        ))?;
    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = to_json(response.into_body()).await?;
    assert_eq!(body["error"]["type"], "invalid_request_error");
    assert_eq!(body["error"]["code"], "model_not_found");
    assert_eq!(body["error"]["param"], "model", "{body}");
    assert!(body["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("other-model"));
    Ok(())
}

#[tokio::test]
async fn endpoints_report_model_param_when_model_is_missing(
) -> Result<(), Box<dyn std::error::Error>> {
    for (uri, payload) in [
        ("/v1/completions", r#"{"prompt":"hello","max_tokens":1}"#),
        (
            "/v1/chat/completions",
            r#"{"messages":[{"role":"user","content":"hello"}],"max_completion_tokens":1}"#,
        ),
    ] {
        let app = router(ServerState::new("fixture-model".to_owned()));
        let request = Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(payload))?;
        let response = app.oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = to_json(response.into_body()).await?;
        assert_eq!(body["error"]["type"], "invalid_request_error");
        assert_eq!(body["error"]["param"], "model", "{body}");
        assert!(body["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("model is required"));
    }
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
    assert_eq!(body["error"]["param"], "prompt", "{body}");
    assert!(body["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("exactly one text prompt"));
    Ok(())
}
