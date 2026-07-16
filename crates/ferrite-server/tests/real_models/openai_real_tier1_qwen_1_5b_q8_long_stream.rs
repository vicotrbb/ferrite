use crate::support;

use std::path::PathBuf;
use support::http::{send_http_request, sse_json_events};

const DEFAULT_MODEL_PATH: &str = "target/models/qwen2.5-1.5b-instruct-q8_0.gguf";
const REAL_MODEL_ID: &str = "qwen2.5-1.5b-q8_0";

#[tokio::test]
#[ignore = "requires local Qwen2.5-1.5B Q8_0 GGUF model artifact"]
async fn live_http_server_streams_chat_with_32_token_limit_and_qwen_1_5b_q8_model()
-> Result<(), Box<dyn std::error::Error>> {
    let server =
        support::LiveServer::start_with_existing_model(REAL_MODEL_ID, qwen_1_5b_q8_model_path()?)
            .await?;
    let request_body = format!(
        r#"{{"model":"{REAL_MODEL_ID}","messages":[{{"role":"user","content":"hello world"}}],"max_completion_tokens":32,"stream":true}}"#
    );
    let response = send_http_request(
        server.addr(),
        "POST",
        "/v1/chat/completions",
        request_body.as_bytes(),
    )
    .await?;

    assert_long_chat_stream_response(&response)?;
    Ok(())
}

fn assert_long_chat_stream_response(response: &str) -> Result<(), Box<dyn std::error::Error>> {
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
        assert_eq!(event["object"], "chat.completion.chunk");
        assert_eq!(event["model"], REAL_MODEL_ID);
    }

    let content_chunks = events
        .iter()
        .filter_map(|event| {
            let choice = &event["choices"][0];
            (choice["finish_reason"].is_null() && choice["delta"]["role"].is_null())
                .then(|| choice["delta"]["content"].as_str())?
        })
        .collect::<Vec<_>>();
    assert!(
        !content_chunks.is_empty(),
        "expected generated content chunks: {events:?}"
    );
    assert!(
        content_chunks.iter().all(|chunk| !chunk.is_empty()),
        "content chunks should not include empty text: {content_chunks:?}"
    );

    let generated_content = content_chunks.join("");
    assert_eq!(
        generated_content, "Hello! How can I help you today?",
        "unexpected deterministic generated content: {events:?}"
    );

    let terminal_chunks = events
        .iter()
        .filter(|event| event["choices"][0]["finish_reason"] == "stop")
        .collect::<Vec<_>>();
    assert_eq!(
        terminal_chunks.len(),
        1,
        "expected exactly one stop terminal chunk"
    );
    assert!(
        terminal_chunks[0]["choices"][0]["delta"]["content"].is_null(),
        "terminal chat stream chunk should not include content"
    );
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
