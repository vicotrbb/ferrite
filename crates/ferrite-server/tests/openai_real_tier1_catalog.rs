mod support;

use std::path::PathBuf;
use support::http::{response_json, send_http_request};
use tokio::sync::Mutex;

const DEFAULT_MODEL_PATH: &str = "target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf";
const REAL_MODEL_ID: &str = "qwen2.5-0.5b-q4_k_m-catalog";
static REAL_MODEL_TEST_LOCK: Mutex<()> = Mutex::const_new(());

#[tokio::test]
#[ignore = "requires local Qwen2.5-0.5B Q4_K_M GGUF model artifact"]
async fn live_http_server_lists_and_retrieves_real_tier1_model(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_lock = REAL_MODEL_TEST_LOCK.lock().await;
    let server =
        support::LiveServer::start_with_existing_model(REAL_MODEL_ID, real_model_path()?).await?;

    let list_response = send_http_request(server.addr(), "GET", "/v1/models", &[]).await?;
    assert!(
        list_response.starts_with("HTTP/1.1 200 OK"),
        "unexpected response: {list_response}"
    );
    let list_body = response_json(&list_response)?;
    assert_eq!(list_body["object"], "list");
    assert_eq!(list_body["data"].as_array().map(Vec::len), Some(1));
    assert_eq!(list_body["data"][0]["id"], REAL_MODEL_ID);
    assert_eq!(list_body["data"][0]["object"], "model");
    assert_eq!(list_body["data"][0]["owned_by"], "ferrite");

    let retrieve_path = format!("/v1/models/{REAL_MODEL_ID}");
    let retrieve_response =
        send_http_request(server.addr(), "GET", retrieve_path.as_str(), &[]).await?;
    assert!(
        retrieve_response.starts_with("HTTP/1.1 200 OK"),
        "unexpected response: {retrieve_response}"
    );
    let retrieve_body = response_json(&retrieve_response)?;
    assert_eq!(retrieve_body["id"], REAL_MODEL_ID);
    assert_eq!(retrieve_body["object"], "model");
    assert_eq!(retrieve_body["owned_by"], "ferrite");

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
