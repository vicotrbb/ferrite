use crate::support;

use std::{net::SocketAddr, path::PathBuf};

use support::http::{response_json, send_http_request};
use tokio::{
    sync::mpsc,
    time::{Duration, sleep},
};

const DEFAULT_MODEL_PATH: &str = "target/models/qwen2.5-1.5b-instruct-q6_k.gguf";
const REAL_MODEL_ID: &str = "qwen2.5-1.5b-q6_k";

#[tokio::test]
#[ignore = "requires local Qwen2.5-1.5B Q6_K GGUF model artifact"]
async fn live_http_server_serves_qwen_1_5b_q6_wait_queue_in_start_order()
-> Result<(), Box<dyn std::error::Error>> {
    let model_path = qwen_1_5b_q6_model_path()?;
    let server = support::LiveServer::start_with_existing_model_configured(
        REAL_MODEL_ID,
        model_path,
        |state| state.with_inference_wait_timeout(Duration::from_secs(300)),
    )
    .await?;
    let holder_body = format!(
        r#"{{"model":"{REAL_MODEL_ID}","messages":[{{"role":"user","content":"hello world"}}],"max_completion_tokens":4,"stream":true}}"#
    );
    let queued_body =
        format!(r#"{{"model":"{REAL_MODEL_ID}","prompt":"hello world","max_tokens":1}}"#);
    let (finish_tx, mut finish_rx) = mpsc::unbounded_channel();

    let holder = spawn_labeled_request(
        server.addr(),
        "/v1/chat/completions",
        holder_body,
        "holder_stream",
        finish_tx.clone(),
    );

    sleep(Duration::from_millis(50)).await;

    let queued_one = spawn_labeled_request(
        server.addr(),
        "/v1/completions",
        queued_body.clone(),
        "queued_one",
        finish_tx.clone(),
    );

    sleep(Duration::from_millis(20)).await;

    let queued_two = spawn_labeled_request(
        server.addr(),
        "/v1/completions",
        queued_body,
        "queued_two",
        finish_tx.clone(),
    );
    drop(finish_tx);

    let holder_response = holder.await??;
    let queued_one_response = queued_one.await??;
    let queued_two_response = queued_two.await??;
    let mut finish_order = Vec::new();
    while let Some(label) = finish_rx.recv().await {
        finish_order.push(label);
    }

    assert_eq!(
        finish_order,
        ["holder_stream", "queued_one", "queued_two"],
        "unexpected finish order"
    );
    assert!(
        holder_response.starts_with("HTTP/1.1 200 OK"),
        "unexpected holder response: {holder_response}"
    );
    assert!(holder_response.contains("data: [DONE]"));
    assert_qwen_1_5b_q6_completion_response(&queued_one_response)?;
    assert_qwen_1_5b_q6_completion_response(&queued_two_response)?;
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

fn assert_qwen_1_5b_q6_completion_response(
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

fn default_model_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(DEFAULT_MODEL_PATH)
}
