use super::routes::router;
use crate::{runtime::InferenceEngine, state::ServerState};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use tower::ServiceExt;

static FIXTURE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[tokio::test]
async fn models_endpoint_returns_empty_list_when_no_model_is_loaded(
) -> Result<(), Box<dyn std::error::Error>> {
    let app = router(ServerState::new("test-model".to_owned()));
    let response = app
        .oneshot(Request::builder().uri("/v1/models").body(Body::empty())?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_json(response.into_body()).await?;
    assert_eq!(body["object"], "list");
    assert!(body["data"].as_array().is_some_and(Vec::is_empty));
    Ok(())
}

#[tokio::test]
async fn models_endpoint_returns_openai_list_shape() -> Result<(), Box<dyn std::error::Error>> {
    let model_path = write_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let app = router(ServerState::with_engine("test-model".to_owned(), engine));
    let response = app
        .oneshot(Request::builder().uri("/v1/models").body(Body::empty())?)
        .await?;
    remove_fixture_model(&model_path)?;

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_json(response.into_body()).await?;
    assert_eq!(body["object"], "list");
    assert_eq!(body["data"][0]["id"], "test-model");
    assert_eq!(body["data"][0]["object"], "model");
    Ok(())
}

#[tokio::test]
async fn model_retrieve_endpoint_rejects_configured_id_when_no_model_is_loaded(
) -> Result<(), Box<dyn std::error::Error>> {
    let app = router(ServerState::new("test-model".to_owned()));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/models/test-model")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = to_json(response.into_body()).await?;
    assert_eq!(body["error"]["type"], "invalid_request_error");
    assert_eq!(body["error"]["code"], "model_not_found");
    Ok(())
}

#[tokio::test]
async fn model_retrieve_endpoint_returns_loaded_model() -> Result<(), Box<dyn std::error::Error>> {
    let model_path = write_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let app = router(ServerState::with_engine("test-model".to_owned(), engine));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/models/test-model")
                .body(Body::empty())?,
        )
        .await?;
    remove_fixture_model(&model_path)?;

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_json(response.into_body()).await?;
    assert_eq!(body["id"], "test-model");
    assert_eq!(body["object"], "model");
    assert_eq!(body["owned_by"], "ferrite");
    Ok(())
}

#[tokio::test]
async fn model_retrieve_endpoint_supports_encoded_slashes() -> Result<(), Box<dyn std::error::Error>>
{
    let model_id = "HuggingFaceTB/SmolLM2-135M-Instruct";
    let model_path = write_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let app = router(ServerState::with_engine(model_id.to_owned(), engine));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/models/HuggingFaceTB%2FSmolLM2-135M-Instruct")
                .body(Body::empty())?,
        )
        .await?;
    remove_fixture_model(&model_path)?;

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_json(response.into_body()).await?;
    assert_eq!(body["id"], model_id);
    assert_eq!(body["object"], "model");
    assert_eq!(body["owned_by"], "ferrite");
    Ok(())
}

#[tokio::test]
async fn model_retrieve_endpoint_rejects_unknown_model() -> Result<(), Box<dyn std::error::Error>> {
    let model_path = write_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let app = router(ServerState::with_engine("test-model".to_owned(), engine));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/models/other-model")
                .body(Body::empty())?,
        )
        .await?;
    remove_fixture_model(&model_path)?;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = to_json(response.into_body()).await?;
    assert_eq!(body["error"]["type"], "invalid_request_error");
    assert_eq!(body["error"]["code"], "model_not_found");
    Ok(())
}

async fn to_json(body: Body) -> Result<Value, Box<dyn std::error::Error>> {
    let bytes = to_bytes(body, usize::MAX).await?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn write_fixture_model() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "ferrite-server-catalog-fixture-{}-{}.gguf",
        std::process::id(),
        FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&path, ferrite_fixtures::scalar_llama_f32_gguf_fixture())?;
    Ok(path)
}

fn remove_fixture_model(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}
