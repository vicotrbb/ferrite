mod support;

use std::path::PathBuf;
use support::http::{response_json, send_http_request};
use tokio::time::{sleep, Duration};

const DEFAULT_MODEL_PATH: &str = "target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf";
const REAL_MODEL_ID: &str = "qwen2.5-0.5b";

#[tokio::test]
#[ignore = "requires local Tier 1 GGUF model artifact"]
async fn live_http_server_generates_with_real_tier1_model() -> Result<(), Box<dyn std::error::Error>>
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
    assert_eq!(body["choices"][0]["text"], "\n");
    assert_eq!(body["usage"]["prompt_tokens"], 2);
    assert_eq!(body["usage"]["completion_tokens"], 1);
    assert_eq!(body["usage"]["total_tokens"], 3);
    Ok(())
}

#[tokio::test]
#[ignore = "requires local Tier 1 GGUF model artifact"]
async fn live_http_server_streams_with_real_tier1_model() -> Result<(), Box<dyn std::error::Error>>
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
    assert!(response.contains("\"text\":\"\\n\""));
    assert!(response.contains("data: [DONE]"));
    Ok(())
}

#[tokio::test]
#[ignore = "requires local Tier 1 GGUF model artifact"]
async fn live_http_server_chats_with_real_tier1_model() -> Result<(), Box<dyn std::error::Error>> {
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
    assert_eq!(body["choices"][0]["message"]["content"], "你好");
    assert_eq!(body["usage"]["prompt_tokens"], 8);
    assert_eq!(body["usage"]["completion_tokens"], 1);
    assert_eq!(body["usage"]["total_tokens"], 9);
    Ok(())
}

#[tokio::test]
#[ignore = "requires local Tier 1 GGUF model artifact"]
async fn live_http_server_streams_chat_with_real_tier1_model(
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
    assert!(response.contains("\"delta\":{\"content\":\"你好\"}"));
    assert!(response.contains("data: [DONE]"));
    Ok(())
}

#[tokio::test]
#[ignore = "requires local Tier 1 GGUF model artifact"]
async fn live_http_server_rejects_concurrent_real_tier1_request(
) -> Result<(), Box<dyn std::error::Error>> {
    let model_path = real_model_path()?;
    let server = support::LiveServer::start_with_existing_model(REAL_MODEL_ID, model_path).await?;
    let addr = server.addr();
    let first_request_body = format!(
        r#"{{"model":"{REAL_MODEL_ID}","messages":[{{"role":"user","content":"hello world"}}],"max_completion_tokens":16,"stream":true}}"#
    );
    let second_request_body =
        format!(r#"{{"model":"{REAL_MODEL_ID}","prompt":"hello world","max_tokens":1}}"#);

    let first_request = tokio::spawn(async move {
        send_http_request(
            addr,
            "POST",
            "/v1/chat/completions",
            first_request_body.as_bytes(),
        )
        .await
        .map_err(|error| error.to_string())
    });

    sleep(Duration::from_millis(50)).await;

    let response = send_http_request(
        server.addr(),
        "POST",
        "/v1/completions",
        second_request_body.as_bytes(),
    )
    .await?;

    assert!(
        response.starts_with("HTTP/1.1 429 Too Many Requests"),
        "unexpected response: {response}"
    );
    let body = response_json(&response)?;
    assert_eq!(body["error"]["type"], "rate_limit_error");
    assert_eq!(
        body["error"]["message"],
        "inference request queue is full; retry later"
    );

    let first_response = first_request.await??;
    assert!(
        first_response.starts_with("HTTP/1.1 200 OK"),
        "unexpected first response: {first_response}"
    );
    assert!(first_response.contains("data: [DONE]"));
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
