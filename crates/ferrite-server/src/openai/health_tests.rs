use super::routes::router;
use super::test_support::{remove_fixture_model, to_json, write_fixture_model};
use crate::{runtime::InferenceEngine, state::ServerState};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

#[tokio::test]
async fn health_endpoint_reports_not_ready_without_loaded_model(
) -> Result<(), Box<dyn std::error::Error>> {
    let app = router(ServerState::new("test-model".to_owned()));
    let response = app
        .oneshot(Request::builder().uri("/health").body(Body::empty())?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_json(response.into_body()).await?;
    assert_eq!(body["status"], "ok");
    assert_eq!(body["model"], "test-model");
    assert_eq!(body["ready"], false);
    Ok(())
}

#[tokio::test]
async fn health_endpoint_reports_ready_with_loaded_model() -> Result<(), Box<dyn std::error::Error>>
{
    let model_path = write_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let app = router(ServerState::with_engine("test-model".to_owned(), engine));
    let response = app
        .oneshot(Request::builder().uri("/health").body(Body::empty())?)
        .await?;
    remove_fixture_model(&model_path)?;

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_json(response.into_body()).await?;
    assert_eq!(body["status"], "ok");
    assert_eq!(body["model"], "test-model");
    assert_eq!(body["ready"], true);
    Ok(())
}
