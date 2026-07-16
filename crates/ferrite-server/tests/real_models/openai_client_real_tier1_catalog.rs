use crate::support;

use std::path::PathBuf;
use support::openai_client::ferrite_client;
use tokio::sync::Mutex;

const DEFAULT_MODEL_PATH: &str = "target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf";
const REAL_MODEL_ID: &str = "qwen2.5-0.5b-q4_k_m-catalog";
const PROVIDER_MODEL_ID: &str = "Qwen/Qwen2.5-0.5B-Instruct-Q4_K_M";
static REAL_MODEL_TEST_LOCK: Mutex<()> = Mutex::const_new(());

#[tokio::test]
#[ignore = "requires local Qwen2.5-0.5B Q4_K_M GGUF model artifact"]
async fn async_openai_client_lists_and_retrieves_real_tier1_model()
-> Result<(), Box<dyn std::error::Error>> {
    let _test_lock = REAL_MODEL_TEST_LOCK.lock().await;
    let server =
        support::LiveServer::start_with_existing_model(REAL_MODEL_ID, real_model_path()?).await?;
    let client = ferrite_client(&server, "local-test");

    let list = client.models().list().await?;
    assert_eq!(list.object, "list");
    assert_eq!(list.data.len(), 1);
    assert_eq!(list.data[0].id, REAL_MODEL_ID);
    assert_eq!(list.data[0].object, "model");
    assert_eq!(list.data[0].owned_by, "ferrite");

    let retrieved = client.models().retrieve(REAL_MODEL_ID).await?;
    assert_eq!(retrieved.id, REAL_MODEL_ID);
    assert_eq!(retrieved.object, "model");
    assert_eq!(retrieved.owned_by, "ferrite");

    Ok(())
}

#[tokio::test]
#[ignore = "requires local Qwen2.5-0.5B Q4_K_M GGUF model artifact"]
async fn async_openai_client_retrieves_real_tier1_provider_style_model_id()
-> Result<(), Box<dyn std::error::Error>> {
    let _test_lock = REAL_MODEL_TEST_LOCK.lock().await;
    let server =
        support::LiveServer::start_with_existing_model(PROVIDER_MODEL_ID, real_model_path()?)
            .await?;
    let client = ferrite_client(&server, "local-test");

    let retrieved = client.models().retrieve(PROVIDER_MODEL_ID).await?;
    assert_eq!(retrieved.id, PROVIDER_MODEL_ID);
    assert_eq!(retrieved.object, "model");
    assert_eq!(retrieved.owned_by, "ferrite");

    Ok(())
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
