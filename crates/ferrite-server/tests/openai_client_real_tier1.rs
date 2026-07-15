mod support;

use std::path::PathBuf;
use support::openai_client::{
    assert_chat_create, assert_chat_stream, assert_completion_create, assert_completion_stream,
};
use tokio::sync::Mutex;

const DEFAULT_MODEL_PATH: &str = "target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf";
const REAL_MODEL_ID: &str = "qwen2.5-0.5b";
const COMPLETION_TEXT: &str = "\n";
const CHAT_CONTENT: &str = "Hello";
static REAL_MODEL_TEST_LOCK: Mutex<()> = Mutex::const_new(());

#[tokio::test]
#[ignore = "requires local Tier 1 GGUF model artifact"]
async fn async_openai_client_generates_with_real_tier1_model(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_lock = REAL_MODEL_TEST_LOCK.lock().await;
    let server =
        support::LiveServer::start_with_existing_model(REAL_MODEL_ID, real_model_path()?).await?;
    assert_completion_create(&server, REAL_MODEL_ID, COMPLETION_TEXT).await
}

#[tokio::test]
#[ignore = "requires local Tier 1 GGUF model artifact"]
async fn async_openai_client_streams_with_real_tier1_model(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_lock = REAL_MODEL_TEST_LOCK.lock().await;
    let server =
        support::LiveServer::start_with_existing_model(REAL_MODEL_ID, real_model_path()?).await?;
    assert_completion_stream(&server, REAL_MODEL_ID, COMPLETION_TEXT).await
}

#[tokio::test]
#[ignore = "requires local Tier 1 GGUF model artifact"]
async fn async_openai_client_chats_with_real_tier1_model() -> Result<(), Box<dyn std::error::Error>>
{
    let _test_lock = REAL_MODEL_TEST_LOCK.lock().await;
    let server =
        support::LiveServer::start_with_existing_model(REAL_MODEL_ID, real_model_path()?).await?;
    assert_chat_create(&server, REAL_MODEL_ID, CHAT_CONTENT).await
}

#[tokio::test]
#[ignore = "requires local Tier 1 GGUF model artifact"]
async fn async_openai_client_streams_chat_with_real_tier1_model(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_lock = REAL_MODEL_TEST_LOCK.lock().await;
    let server =
        support::LiveServer::start_with_existing_model(REAL_MODEL_ID, real_model_path()?).await?;
    assert_chat_stream(&server, REAL_MODEL_ID, CHAT_CONTENT).await
}

fn real_model_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let model_path = std::env::var_os("FERRITE_REAL_TIER1_MODEL")
        .map(PathBuf::from)
        .unwrap_or_else(default_model_path);
    if !model_path.is_file() {
        return Err(format!(
            "missing real Tier 1 model artifact: {}",
            model_path.display()
        )
        .into());
    }
    Ok(model_path)
}

fn default_model_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(DEFAULT_MODEL_PATH)
}
