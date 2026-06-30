mod support;

use std::{net::SocketAddr, path::PathBuf};

use support::http::{response_json, send_http_request};
use tokio::{
    sync::mpsc,
    time::{sleep, Duration},
};

const DEFAULT_MODEL_PATH: &str = "target/models/qwen2.5-1.5b-instruct-q8_0.gguf";
const REAL_MODEL_ID: &str = "qwen2.5-1.5b-q8_0";

#[tokio::test]
#[ignore = "requires local Qwen2.5-1.5B Q8_0 GGUF model artifact"]
async fn live_http_server_queues_behind_32_token_qwen_1_5b_q8_stream(
) -> Result<(), Box<dyn std::error::Error>> {
    let server = support::LiveServer::start_with_existing_model_configured(
        REAL_MODEL_ID,
        qwen_1_5b_q8_model_path()?,
        |state| state.with_inference_wait_timeout(Duration::from_secs(300)),
    )
    .await?;
    let holder_body = format!(
        r#"{{"model":"{REAL_MODEL_ID}","messages":[{{"role":"user","content":"hello world"}}],"max_completion_tokens":32,"stream":true}}"#
    );
    let queued_chat_body = format!(
        r#"{{"model":"{REAL_MODEL_ID}","messages":[{{"role":"user","content":"hello world"}}],"max_completion_tokens":32}}"#
    );
    let queued_completion_body =
        format!(r#"{{"model":"{REAL_MODEL_ID}","prompt":"hello world","max_tokens":1}}"#);
    let (finish_tx, mut finish_rx) = mpsc::unbounded_channel();

    let holder_stream = spawn_labeled_request(
        server.addr(),
        "/v1/chat/completions",
        holder_body,
        "holder_stream",
        finish_tx.clone(),
    );

    sleep(Duration::from_millis(50)).await;

    let queued_chat = spawn_labeled_request(
        server.addr(),
        "/v1/chat/completions",
        queued_chat_body,
        "queued_chat",
        finish_tx.clone(),
    );

    sleep(Duration::from_millis(20)).await;

    let queued_completion = spawn_labeled_request(
        server.addr(),
        "/v1/completions",
        queued_completion_body,
        "queued_completion",
        finish_tx.clone(),
    );
    drop(finish_tx);

    let holder_response = holder_stream.await??;
    let queued_chat_response = queued_chat.await??;
    let queued_completion_response = queued_completion.await??;
    let mut finish_order = Vec::new();
    while let Some(label) = finish_rx.recv().await {
        finish_order.push(label);
    }

    assert_eq!(
        finish_order,
        ["holder_stream", "queued_chat", "queued_completion"],
        "unexpected finish order"
    );
    assert_qwen_1_5b_q8_32_token_chat_stream_response(&holder_response)?;
    assert_qwen_1_5b_q8_32_token_chat_response(&queued_chat_response)?;
    assert_qwen_1_5b_q8_completion_response(&queued_completion_response)?;
    Ok(())
}

fn spawn_labeled_request(
    addr: SocketAddr,
    path: &'static str,
    body: String,
    label: &'static str,
    finish_tx: mpsc::UnboundedSender<&'static str>,
) -> tokio::task::JoinHandle<Result<String, String>> {
    tokio::spawn(async move {
        let response = send_http_request(addr, "POST", path, body.as_bytes())
            .await
            .map_err(|error| format!("{label}: {error}"))?;
        finish_tx.send(label).map_err(|error| error.to_string())?;
        Ok(response)
    })
}

fn assert_qwen_1_5b_q8_32_token_chat_stream_response(
    response: &str,
) -> Result<(), Box<dyn std::error::Error>> {
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
    assert!(response.contains("data: [DONE]"), "missing SSE terminator");
    assert!(
        response.contains("\"object\":\"chat.completion.chunk\""),
        "missing chat stream chunk: {response}"
    );
    assert!(
        response.contains("\"finish_reason\":\"length\""),
        "missing length terminal chunk: {response}"
    );
    Ok(())
}

fn assert_qwen_1_5b_q8_32_token_chat_response(
    response: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "unexpected response: {response}"
    );
    let body = response_json(response)?;
    assert_eq!(body["object"], "chat.completion");
    assert_eq!(body["model"], REAL_MODEL_ID);
    assert_eq!(body["choices"][0]["finish_reason"], "length");
    assert!(
        body["choices"][0]["message"]["content"]
            .as_str()
            .is_some_and(|content| !content.is_empty()),
        "unexpected response body: {body}"
    );
    assert_eq!(body["usage"]["prompt_tokens"], 8);
    assert_eq!(body["usage"]["completion_tokens"], 32);
    assert_eq!(body["usage"]["total_tokens"], 40);
    Ok(())
}

fn assert_qwen_1_5b_q8_completion_response(
    response: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "unexpected response: {response}"
    );
    let body = response_json(response)?;
    assert_eq!(body["object"], "text_completion");
    assert_eq!(body["model"], REAL_MODEL_ID);
    assert_eq!(body["choices"][0]["text"], "\n");
    assert_eq!(body["usage"]["prompt_tokens"], 2);
    assert_eq!(body["usage"]["completion_tokens"], 1);
    assert_eq!(body["usage"]["total_tokens"], 3);
    Ok(())
}

fn qwen_1_5b_q8_model_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let model_path = std::env::var_os("FERRITE_QWEN_1_5B_Q8_MODEL")
        .map(PathBuf::from)
        .unwrap_or_else(default_model_path);
    if !model_path.is_file() {
        return Err(format!(
            "missing Qwen2.5-1.5B Q8_0 model artifact: {}",
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
