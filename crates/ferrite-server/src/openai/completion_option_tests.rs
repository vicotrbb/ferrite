use super::routes::router;
use super::test_support::{remove_fixture_model, to_json, write_fixture_model};
use crate::{runtime::InferenceEngine, state::ServerState};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

#[tokio::test]
async fn completions_endpoint_accepts_neutral_sampling_options(
) -> Result<(), Box<dyn std::error::Error>> {
    assert_completion_option_is_accepted(
        r#""temperature":0,"top_p":1,"n":1,"presence_penalty":0,"frequency_penalty":0"#,
    )
    .await
}

#[tokio::test]
async fn completions_endpoint_accepts_openai_default_temperature(
) -> Result<(), Box<dyn std::error::Error>> {
    assert_completion_option_is_accepted(r#""temperature":1,"top_k":1"#).await
}

#[tokio::test]
async fn completions_endpoint_applies_logit_bias() -> Result<(), Box<dyn std::error::Error>> {
    let body =
        accepted_completion_option_response(r#""temperature":0,"logit_bias":{"1":100}"#).await?;
    assert_eq!(body["choices"][0]["text"], "hello");
    Ok(())
}

#[tokio::test]
async fn completions_endpoint_accepts_extended_sampling_controls(
) -> Result<(), Box<dyn std::error::Error>> {
    assert_completion_option_is_accepted(
        r#""temperature":0.8,"top_k":1,"top_p":0.9,"min_p":0.05,"repetition_penalty":1.1,"presence_penalty":0.2,"frequency_penalty":-0.2,"seed":42"#,
    )
    .await
}

#[tokio::test]
async fn completions_endpoint_accepts_empty_stop_array() -> Result<(), Box<dyn std::error::Error>> {
    assert_completion_option_is_accepted(r#""stop":[]"#).await
}

#[tokio::test]
async fn completions_endpoint_accepts_disabled_echo() -> Result<(), Box<dyn std::error::Error>> {
    assert_completion_option_is_accepted(r#""echo":false"#).await
}

#[tokio::test]
async fn completions_endpoint_accepts_null_echo() -> Result<(), Box<dyn std::error::Error>> {
    assert_completion_option_is_accepted(r#""echo":null"#).await
}

#[tokio::test]
async fn completions_endpoint_echoes_prompt_when_requested(
) -> Result<(), Box<dyn std::error::Error>> {
    let body = accepted_completion_option_response(r#""echo":true"#).await?;
    assert_eq!(body["choices"][0]["text"], "hellowinner");
    Ok(())
}

#[tokio::test]
async fn completions_endpoint_accepts_single_best_of_candidate(
) -> Result<(), Box<dyn std::error::Error>> {
    assert_completion_option_is_accepted(r#""best_of":1"#).await
}

#[tokio::test]
async fn completions_endpoint_accepts_empty_logit_bias() -> Result<(), Box<dyn std::error::Error>> {
    assert_completion_option_is_accepted(r#""logit_bias":{}"#).await
}

#[tokio::test]
async fn completions_endpoint_accepts_null_logprobs() -> Result<(), Box<dyn std::error::Error>> {
    assert_completion_option_is_accepted(r#""logprobs":null"#).await
}

#[tokio::test]
async fn completions_endpoint_accepts_user_identifier() -> Result<(), Box<dyn std::error::Error>> {
    assert_completion_option_is_accepted(r#""user":"local-user-1""#).await
}

#[tokio::test]
async fn completions_endpoint_accepts_seed() -> Result<(), Box<dyn std::error::Error>> {
    let body = accepted_completion_option_response(r#""seed":42"#).await?;
    assert_eq!(body["choices"][0]["text"], "winner");
    assert!(body["system_fingerprint"].is_null(), "{body}");
    Ok(())
}

async fn assert_completion_option_is_accepted(
    option_json: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let body = accepted_completion_option_response(option_json).await?;
    assert_eq!(body["choices"][0]["text"], "winner");
    Ok(())
}

async fn accepted_completion_option_response(
    option_json: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let model_path = write_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let app = router(ServerState::with_engine("fixture-model".to_owned(), engine));
    let body =
        format!(r#"{{"model":"fixture-model","prompt":"hello","max_tokens":1,{option_json}}}"#);
    let request = Request::builder()
        .method("POST")
        .uri("/v1/completions")
        .header("content-type", "application/json")
        .body(Body::from(body))?;
    let response = app.oneshot(request).await?;
    remove_fixture_model(&model_path)?;

    let status = response.status();
    let body = to_json(response.into_body()).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    Ok(body)
}
