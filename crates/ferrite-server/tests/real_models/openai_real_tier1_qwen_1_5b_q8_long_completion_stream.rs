use crate::support;

use std::path::PathBuf;
use support::http::{send_http_request, sse_json_events};

const DEFAULT_MODEL_PATH: &str = "target/models/qwen2.5-1.5b-instruct-q8_0.gguf";
const REAL_MODEL_ID: &str = "qwen2.5-1.5b-q8_0";

#[tokio::test]
#[ignore = "requires local Qwen2.5-1.5B Q8_0 GGUF model artifact"]
async fn live_http_server_streams_32_token_completion_with_qwen_1_5b_q8_model()
-> Result<(), Box<dyn std::error::Error>> {
    let server =
        support::LiveServer::start_with_existing_model(REAL_MODEL_ID, qwen_1_5b_q8_model_path()?)
            .await?;
    let request_body = format!(
        r#"{{"model":"{REAL_MODEL_ID}","prompt":"hello world","max_tokens":32,"stream":true}}"#
    );
    let response = send_http_request(
        server.addr(),
        "POST",
        "/v1/completions",
        request_body.as_bytes(),
    )
    .await?;

    assert_long_completion_stream_response(&response)?;
    Ok(())
}

fn assert_long_completion_stream_response(
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
    assert!(
        response.contains("data: [DONE]"),
        "missing stream terminator: {response}"
    );

    let events = sse_json_events(response)?;
    assert!(!events.is_empty(), "missing JSON SSE events: {response}");
    for event in &events {
        assert_eq!(event["object"], "text_completion");
        assert_eq!(event["model"], REAL_MODEL_ID);
    }

    let text_chunks = events
        .iter()
        .filter_map(|event| {
            let choice = &event["choices"][0];
            (choice["finish_reason"].is_null()).then(|| choice["text"].as_str())?
        })
        .collect::<Vec<_>>();
    assert!(
        !text_chunks.is_empty(),
        "missing generated chunks: {events:?}"
    );
    assert!(
        text_chunks.iter().all(|chunk| !chunk.is_empty()),
        "completion stream should not emit empty text chunks: {text_chunks:?}"
    );

    let generated_text = text_chunks.join("");
    assert!(!generated_text.is_empty(), "missing generated text");

    let terminal_chunks = events
        .iter()
        .filter(|event| event["choices"][0]["finish_reason"] == "length")
        .collect::<Vec<_>>();
    assert_eq!(
        terminal_chunks.len(),
        1,
        "expected exactly one length terminal chunk"
    );
    assert_eq!(terminal_chunks[0]["choices"][0]["text"], "");
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
