use crate::support;

use std::path::PathBuf;
use support::openai_client::{
    assert_chat_create, assert_chat_stream, assert_completion_create, assert_completion_stream,
};

const DEFAULT_MODEL_PATH: &str = "target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf";
const REAL_MODEL_ID: &str = "smollm2-135m";
const COMPLETION_TEXT: &str = ".";
const CHAT_CONTENT: &str = "Hello";

#[tokio::test]
#[ignore = "requires local Tier 0 GGUF model artifact"]
async fn async_openai_client_generates_with_real_tier0_model()
-> Result<(), Box<dyn std::error::Error>> {
    let server =
        support::LiveServer::start_with_existing_model(REAL_MODEL_ID, real_model_path()?).await?;
    assert_completion_create(&server, REAL_MODEL_ID, COMPLETION_TEXT).await
}

#[tokio::test]
#[ignore = "requires local Tier 0 GGUF model artifact"]
async fn async_openai_client_streams_with_real_tier0_model()
-> Result<(), Box<dyn std::error::Error>> {
    let server =
        support::LiveServer::start_with_existing_model(REAL_MODEL_ID, real_model_path()?).await?;
    assert_completion_stream(&server, REAL_MODEL_ID, COMPLETION_TEXT).await
}

#[tokio::test]
#[ignore = "requires local Tier 0 GGUF model artifact"]
async fn async_openai_client_chats_with_real_tier0_model() -> Result<(), Box<dyn std::error::Error>>
{
    let server =
        support::LiveServer::start_with_existing_model(REAL_MODEL_ID, real_model_path()?).await?;
    assert_chat_create(&server, REAL_MODEL_ID, CHAT_CONTENT).await
}

#[tokio::test]
#[ignore = "requires local Tier 0 GGUF model artifact"]
async fn async_openai_client_streams_chat_with_real_tier0_model()
-> Result<(), Box<dyn std::error::Error>> {
    let server =
        support::LiveServer::start_with_existing_model(REAL_MODEL_ID, real_model_path()?).await?;
    assert_chat_stream(&server, REAL_MODEL_ID, CHAT_CONTENT).await
}

fn real_model_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let model_path = std::env::var_os("FERRITE_REAL_MODEL")
        .map(PathBuf::from)
        .unwrap_or_else(default_model_path);
    if !model_path.is_file() {
        return Err(format!("missing real model artifact: {}", model_path.display()).into());
    }
    Ok(model_path)
}

fn default_model_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(DEFAULT_MODEL_PATH)
}
