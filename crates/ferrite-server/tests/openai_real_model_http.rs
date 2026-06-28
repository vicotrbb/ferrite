mod support;

use std::path::PathBuf;
use support::http::{response_json, send_http_request};

const DEFAULT_MODEL_PATH: &str = "target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf";
const REAL_MODEL_ID: &str = "smollm2-135m";

#[tokio::test]
#[ignore = "requires local Tier 0 GGUF model artifact"]
async fn live_http_server_generates_with_real_tier0_model() -> Result<(), Box<dyn std::error::Error>>
{
    let model_path = real_model_path()?;
    let server = support::LiveServer::start_with_existing_model(REAL_MODEL_ID, model_path).await?;
    let request_body =
        format!(r#"{{"model":"{REAL_MODEL_ID}","prompt":"hello world","max_tokens":1}}"#);
    let response = send_http_request(
        server.addr(),
        "POST",
        "/v1/completions",
        request_body.as_bytes(),
    )
    .await?;

    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "unexpected response: {response}"
    );
    let body = response_json(&response)?;
    assert_eq!(body["object"], "text_completion");
    assert_eq!(body["model"], REAL_MODEL_ID);
    assert_eq!(body["choices"][0]["text"], ".");
    assert_eq!(body["usage"]["prompt_tokens"], 2);
    assert_eq!(body["usage"]["completion_tokens"], 1);
    assert_eq!(body["usage"]["total_tokens"], 3);
    Ok(())
}

#[tokio::test]
#[ignore = "requires local Tier 0 GGUF model artifact"]
async fn live_http_server_streams_with_real_tier0_model() -> Result<(), Box<dyn std::error::Error>>
{
    let model_path = real_model_path()?;
    let server = support::LiveServer::start_with_existing_model(REAL_MODEL_ID, model_path).await?;
    let request_body = format!(
        r#"{{"model":"{REAL_MODEL_ID}","prompt":"hello world","max_tokens":1,"stream":true}}"#
    );
    let response = send_http_request(
        server.addr(),
        "POST",
        "/v1/completions",
        request_body.as_bytes(),
    )
    .await?;

    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "unexpected response: {response}"
    );
    assert!(
        response
            .to_ascii_lowercase()
            .contains("content-type: text/event-stream"),
        "unexpected response headers: {response}"
    );
    assert!(response.contains("data: {\"id\":\"cmpl-ferrite-"));
    assert!(response.contains("\"object\":\"text_completion\""));
    assert!(response.contains("\"text\":\".\""));
    assert!(response.contains("data: [DONE]"));
    Ok(())
}

#[tokio::test]
#[ignore = "requires local Tier 0 GGUF model artifact"]
async fn live_http_server_chats_with_real_tier0_model() -> Result<(), Box<dyn std::error::Error>> {
    let model_path = real_model_path()?;
    let server = support::LiveServer::start_with_existing_model(REAL_MODEL_ID, model_path).await?;
    let request_body = format!(
        r#"{{"model":"{REAL_MODEL_ID}","messages":[{{"role":"user","content":"hello world"}}],"max_completion_tokens":1}}"#
    );
    let response = send_http_request(
        server.addr(),
        "POST",
        "/v1/chat/completions",
        request_body.as_bytes(),
    )
    .await?;

    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "unexpected response: {response}"
    );
    let body = response_json(&response)?;
    assert_eq!(body["object"], "chat.completion");
    assert_eq!(body["model"], REAL_MODEL_ID);
    assert_eq!(body["choices"][0]["message"]["content"], "\n");
    assert_eq!(body["usage"]["prompt_tokens"], 9);
    assert_eq!(body["usage"]["completion_tokens"], 1);
    assert_eq!(body["usage"]["total_tokens"], 10);
    Ok(())
}

#[tokio::test]
#[ignore = "requires local Tier 0 GGUF model artifact"]
async fn live_http_server_streams_chat_with_real_tier0_model(
) -> Result<(), Box<dyn std::error::Error>> {
    let model_path = real_model_path()?;
    let server = support::LiveServer::start_with_existing_model(REAL_MODEL_ID, model_path).await?;
    let request_body = format!(
        r#"{{"model":"{REAL_MODEL_ID}","messages":[{{"role":"user","content":"hello world"}}],"max_completion_tokens":1,"stream":true}}"#
    );
    let response = send_http_request(
        server.addr(),
        "POST",
        "/v1/chat/completions",
        request_body.as_bytes(),
    )
    .await?;

    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "unexpected response: {response}"
    );
    assert!(
        response
            .to_ascii_lowercase()
            .contains("content-type: text/event-stream"),
        "unexpected response headers: {response}"
    );
    assert!(response.contains("data: {\"id\":\"chatcmpl-ferrite-"));
    assert!(response.contains("\"object\":\"chat.completion.chunk\""));
    assert!(response.contains("\"delta\":{\"content\":\"\\n\"}"));
    assert!(response.contains("data: [DONE]"));
    Ok(())
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
