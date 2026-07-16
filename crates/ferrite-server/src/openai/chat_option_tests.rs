use super::routes::router;
use super::test_support::{remove_fixture_model, to_json, write_chat_fixture_model};
use crate::{runtime::InferenceEngine, state::ServerState};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

#[tokio::test]
async fn chat_endpoint_accepts_neutral_sampling_options() -> Result<(), Box<dyn std::error::Error>>
{
    assert_chat_option_is_accepted(
        r#""temperature":0,"top_p":1,"n":1,"presence_penalty":0,"frequency_penalty":0"#,
    )
    .await
}

#[tokio::test]
async fn chat_endpoint_accepts_openai_default_temperature() -> Result<(), Box<dyn std::error::Error>>
{
    assert_chat_option_is_accepted(r#""temperature":1,"top_k":1"#).await
}

#[tokio::test]
async fn chat_endpoint_applies_logit_bias() -> Result<(), Box<dyn std::error::Error>> {
    let body = accepted_chat_option_response(r#""temperature":0,"logit_bias":{"1":100}"#).await?;
    assert_eq!(body["choices"][0]["message"]["content"], "hello");
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_rejects_out_of_vocabulary_logit_bias()
-> Result<(), Box<dyn std::error::Error>> {
    let model_path = write_chat_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let app = router(ServerState::with_engine("fixture-model".to_owned(), engine));
    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"model":"fixture-model","messages":[{"role":"user","content":"hello"}],"max_completion_tokens":1,"logit_bias":{"999999":1}}"#,
        ))?;
    let response = app.oneshot(request).await?;
    remove_fixture_model(&model_path)?;

    let status = response.status();
    let body = to_json(response.into_body()).await?;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["error"]["param"], "logit_bias");
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("vocabulary size")
    );
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_accepts_extended_sampling_controls() -> Result<(), Box<dyn std::error::Error>>
{
    assert_chat_option_is_accepted(
        r#""temperature":0.8,"top_k":1,"top_p":0.9,"min_p":0.05,"repetition_penalty":1.1,"presence_penalty":0.2,"frequency_penalty":-0.2,"seed":42"#,
    )
    .await
}

#[tokio::test]
async fn chat_endpoint_accepts_empty_stop_array() -> Result<(), Box<dyn std::error::Error>> {
    assert_chat_option_is_accepted(r#""stop":[]"#).await
}

#[tokio::test]
async fn chat_endpoint_accepts_disabled_logprobs_and_store()
-> Result<(), Box<dyn std::error::Error>> {
    assert_chat_option_is_accepted(r#""logprobs":false,"store":false"#).await
}

#[tokio::test]
async fn chat_endpoint_accepts_empty_logit_bias() -> Result<(), Box<dyn std::error::Error>> {
    assert_chat_option_is_accepted(r#""logit_bias":{}"#).await
}

#[tokio::test]
async fn chat_endpoint_accepts_text_response_format() -> Result<(), Box<dyn std::error::Error>> {
    assert_chat_option_is_accepted(r#""response_format":{"type":"text"}"#).await
}

#[tokio::test]
async fn chat_endpoint_accepts_null_response_format() -> Result<(), Box<dyn std::error::Error>> {
    assert_chat_option_is_accepted(r#""response_format":null"#).await
}

#[tokio::test]
async fn chat_endpoint_accepts_text_only_modalities() -> Result<(), Box<dyn std::error::Error>> {
    assert_chat_option_is_accepted(r#""modalities":["text"]"#).await
}

#[tokio::test]
async fn chat_endpoint_accepts_null_modalities() -> Result<(), Box<dyn std::error::Error>> {
    assert_chat_option_is_accepted(r#""modalities":null"#).await
}

#[tokio::test]
async fn chat_endpoint_accepts_null_optional_openai_options()
-> Result<(), Box<dyn std::error::Error>> {
    assert_chat_option_is_accepted(
        r#""audio":null,"moderation":null,"prediction":null,"verbosity":null,"web_search_options":null"#,
    )
    .await
}

#[tokio::test]
async fn chat_endpoint_accepts_explicit_no_tool_options() -> Result<(), Box<dyn std::error::Error>>
{
    assert_chat_option_is_accepted(r#""tools":[],"tool_choice":"none","parallel_tool_calls":false"#)
        .await
}

#[tokio::test]
async fn chat_endpoint_accepts_auto_tool_choice_without_tools()
-> Result<(), Box<dyn std::error::Error>> {
    assert_chat_option_is_accepted(r#""tool_choice":"auto""#).await
}

#[tokio::test]
async fn chat_endpoint_accepts_parallel_tool_calls_without_tools()
-> Result<(), Box<dyn std::error::Error>> {
    assert_chat_option_is_accepted(r#""tools":[],"parallel_tool_calls":true"#).await
}

#[tokio::test]
async fn chat_endpoint_accepts_explicit_no_function_options()
-> Result<(), Box<dyn std::error::Error>> {
    assert_chat_option_is_accepted(r#""functions":[],"function_call":"none""#).await
}

#[tokio::test]
async fn chat_endpoint_accepts_auto_function_call_without_functions()
-> Result<(), Box<dyn std::error::Error>> {
    assert_chat_option_is_accepted(r#""function_call":"auto""#).await
}

#[tokio::test]
async fn chat_endpoint_accepts_user_identifier() -> Result<(), Box<dyn std::error::Error>> {
    assert_chat_option_is_accepted(r#""user":"local-user-1""#).await
}

#[tokio::test]
async fn chat_endpoint_accepts_seed() -> Result<(), Box<dyn std::error::Error>> {
    let body = accepted_chat_option_response(r#""seed":42"#).await?;
    assert_eq!(body["choices"][0]["message"]["content"], "winner");
    assert!(body["system_fingerprint"].is_null(), "{body}");
    Ok(())
}

#[tokio::test]
async fn chat_endpoint_accepts_metadata_object() -> Result<(), Box<dyn std::error::Error>> {
    assert_chat_option_is_accepted(r#""metadata":{"trace_id":"local-123","tenant":"dev"}"#).await
}

#[tokio::test]
async fn chat_endpoint_accepts_prompt_cache_key() -> Result<(), Box<dyn std::error::Error>> {
    assert_chat_option_is_accepted(r#""prompt_cache_key":"tenant-a:prompt-1""#).await
}

#[tokio::test]
async fn chat_endpoint_accepts_safety_identifier() -> Result<(), Box<dyn std::error::Error>> {
    assert_chat_option_is_accepted(r#""safety_identifier":"hashed-local-user""#).await
}

#[tokio::test]
async fn chat_endpoint_accepts_no_reasoning_effort() -> Result<(), Box<dyn std::error::Error>> {
    assert_chat_option_is_accepted(r#""reasoning_effort":"none""#).await
}

#[tokio::test]
async fn chat_endpoint_accepts_return_token_ids_extension() -> Result<(), Box<dyn std::error::Error>>
{
    assert_chat_option_is_accepted(r#""return_token_ids":true"#).await
}

async fn assert_chat_option_is_accepted(
    option_json: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let body = accepted_chat_option_response(option_json).await?;
    assert_eq!(body["choices"][0]["message"]["content"], "winner");
    Ok(())
}

async fn accepted_chat_option_response(
    option_json: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let model_path = write_chat_fixture_model()?;
    let engine = InferenceEngine::load(&model_path)?;
    let app = router(ServerState::with_engine("fixture-model".to_owned(), engine));
    let body = format!(
        r#"{{"model":"fixture-model","messages":[{{"role":"user","content":"hello"}}],"max_completion_tokens":1,{option_json}}}"#
    );
    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .body(Body::from(body))?;
    let response = app.oneshot(request).await?;
    remove_fixture_model(&model_path)?;

    let status = response.status();
    let body = to_json(response.into_body()).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    Ok(body)
}
