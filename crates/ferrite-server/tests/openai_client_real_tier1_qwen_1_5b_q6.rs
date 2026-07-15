mod support;

use std::path::PathBuf;
use support::openai_client::{
    assert_chat_create, assert_chat_stream, assert_completion_create, assert_completion_stream,
};
use tokio::sync::Mutex;

const DEFAULT_MODEL_PATH: &str = "target/models/qwen2.5-1.5b-instruct-q6_k.gguf";
const REAL_MODEL_ID: &str = "qwen2.5-1.5b-q6_k";
const COMPLETION_TEXT: &str = "\n";
const CHAT_CONTENT: &str = "Hello";
static REAL_MODEL_TEST_LOCK: Mutex<()> = Mutex::const_new(());

#[tokio::test]
#[ignore = "requires local Qwen2.5-1.5B Q6_K GGUF model artifact"]
async fn async_openai_client_generates_with_qwen_1_5b_q6_model(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_lock = REAL_MODEL_TEST_LOCK.lock().await;
    let server =
        support::LiveServer::start_with_existing_model(REAL_MODEL_ID, qwen_1_5b_q6_model_path()?)
            .await?;

    assert_completion_create(&server, REAL_MODEL_ID, COMPLETION_TEXT).await
}

#[tokio::test]
#[ignore = "requires local Qwen2.5-1.5B Q6_K GGUF model artifact"]
async fn async_openai_client_streams_with_qwen_1_5b_q6_model(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_lock = REAL_MODEL_TEST_LOCK.lock().await;
    let server =
        support::LiveServer::start_with_existing_model(REAL_MODEL_ID, qwen_1_5b_q6_model_path()?)
            .await?;

    assert_completion_stream(&server, REAL_MODEL_ID, COMPLETION_TEXT).await
}

#[tokio::test]
#[ignore = "requires local Qwen2.5-1.5B Q6_K GGUF model artifact"]
async fn async_openai_client_chats_with_qwen_1_5b_q6_model(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_lock = REAL_MODEL_TEST_LOCK.lock().await;
    let server =
        support::LiveServer::start_with_existing_model(REAL_MODEL_ID, qwen_1_5b_q6_model_path()?)
            .await?;

    assert_chat_create(&server, REAL_MODEL_ID, CHAT_CONTENT).await
}

#[tokio::test]
#[ignore = "requires local Qwen2.5-1.5B Q6_K GGUF model artifact"]
async fn async_openai_client_streams_chat_with_qwen_1_5b_q6_model(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_lock = REAL_MODEL_TEST_LOCK.lock().await;
    let server =
        support::LiveServer::start_with_existing_model(REAL_MODEL_ID, qwen_1_5b_q6_model_path()?)
            .await?;

    assert_chat_stream(&server, REAL_MODEL_ID, CHAT_CONTENT).await
}

fn qwen_1_5b_q6_model_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let model_path = std::env::var_os("FERRITE_QWEN_1_5B_Q6_MODEL")
        .map(PathBuf::from)
        .unwrap_or_else(default_model_path);
    if !model_path.is_file() {
        return Err(format!(
            "missing Qwen2.5-1.5B Q6_K model artifact: {}",
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
