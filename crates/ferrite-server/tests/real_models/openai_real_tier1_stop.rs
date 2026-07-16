use crate::support;

use std::path::PathBuf;
use support::{
    http::{response_json, send_http_request},
    stop_sequences::{assert_stop_chat_stream, assert_stop_completion_stream},
};

const DEFAULT_MODEL_PATH: &str = "target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf";
const REAL_MODEL_ID: &str = "qwen2.5-0.5b";

#[tokio::test]
#[ignore = "requires local Tier 1 GGUF model artifact"]
async fn live_http_server_applies_stop_sequences_with_real_tier1_model()
-> Result<(), Box<dyn std::error::Error>> {
    let model_path = real_model_path()?;
    let server = support::LiveServer::start_with_existing_model(REAL_MODEL_ID, model_path).await?;

    let completion_body = format!(
        r#"{{"model":"{REAL_MODEL_ID}","prompt":"hello world","max_tokens":1,"stop":"\n"}}"#
    );
    let completion_response = send_http_request(
        server.addr(),
        "POST",
        "/v1/completions",
        completion_body.as_bytes(),
    )
    .await?;

    assert!(
        completion_response.starts_with("HTTP/1.1 200 OK"),
        "unexpected completion response: {completion_response}"
    );
    let completion_body = response_json(&completion_response)?;
    assert_eq!(completion_body["choices"][0]["text"], "");
    assert_eq!(completion_body["usage"]["completion_tokens"], 1);

    let chat_body = format!(
        r#"{{"model":"{REAL_MODEL_ID}","messages":[{{"role":"user","content":"hello world"}}],"max_completion_tokens":1,"stop":"Hello"}}"#
    );
    let chat_response = send_http_request(
        server.addr(),
        "POST",
        "/v1/chat/completions",
        chat_body.as_bytes(),
    )
    .await?;

    assert!(
        chat_response.starts_with("HTTP/1.1 200 OK"),
        "unexpected chat response: {chat_response}"
    );
    let chat_body = response_json(&chat_response)?;
    assert_eq!(chat_body["choices"][0]["message"]["content"], "");
    assert_eq!(chat_body["usage"]["completion_tokens"], 1);
    Ok(())
}

#[tokio::test]
#[ignore = "requires local Tier 1 GGUF model artifact"]
async fn live_http_server_streams_stop_sequences_with_real_tier1_model()
-> Result<(), Box<dyn std::error::Error>> {
    let model_path = real_model_path()?;
    let server = support::LiveServer::start_with_existing_model(REAL_MODEL_ID, model_path).await?;

    let completion_body = format!(
        r#"{{"model":"{REAL_MODEL_ID}","prompt":"hello world","max_tokens":1,"stream":true,"stop":"\n"}}"#
    );
    let completion_response = send_http_request(
        server.addr(),
        "POST",
        "/v1/completions",
        completion_body.as_bytes(),
    )
    .await?;

    assert_stop_completion_stream(&completion_response)?;

    let chat_body = format!(
        r#"{{"model":"{REAL_MODEL_ID}","messages":[{{"role":"user","content":"hello world"}}],"max_completion_tokens":1,"stream":true,"stop":"Hello"}}"#
    );
    let chat_response = send_http_request(
        server.addr(),
        "POST",
        "/v1/chat/completions",
        chat_body.as_bytes(),
    )
    .await?;

    assert_stop_chat_stream(&chat_response)?;
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
