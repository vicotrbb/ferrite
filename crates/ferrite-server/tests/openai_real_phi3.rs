mod support;

use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};
use support::http::{response_json, send_http_request, sse_json_events};

const REAL_MODEL_ID: &str = "phi3-mini-4k-instruct-q4";
const EXPECTED_MODEL_BYTES: u64 = 2_393_231_072;
const EXPECTED_MODEL_SHA256: &str =
    "8a83c7fb9049a9b2e92266fa7ad04933bb53aa1e85136b7b30f1b8000ff2edef";
const PROMPT: &str = "Write one word about iron.";
const EXPECTED_VISIBLE_TOKEN_IDS: &[u64] = &[2443, 295];
const EXPECTED_CONTENT: &str = " Steel";

#[tokio::test]
#[ignore = "requires the pinned official Phi-3 Mini 4K Instruct Q4 GGUF artifact"]
async fn live_phi3_chat_stops_on_model_native_end_token() -> Result<(), Box<dyn std::error::Error>>
{
    let model_path = verified_phi3_model_path()?;
    let server = support::LiveServer::start_with_existing_model(REAL_MODEL_ID, model_path).await?;

    let response = send_http_request(
        server.addr(),
        "POST",
        "/v1/chat/completions",
        chat_body(false).as_bytes(),
    )
    .await?;
    assert_non_streaming_response(&response)?;

    let response = send_http_request(
        server.addr(),
        "POST",
        "/v1/chat/completions",
        chat_body(true).as_bytes(),
    )
    .await?;
    assert_streaming_response(&response)?;
    Ok(())
}

fn chat_body(stream: bool) -> String {
    serde_json::json!({
        "model": REAL_MODEL_ID,
        "messages": [
            {
                "role": "user",
                "content": PROMPT,
            },
        ],
        "max_completion_tokens": 16,
        "stream": stream,
        "stream_options": stream.then_some(serde_json::json!({
            "include_usage": true,
            "include_obfuscation": false,
        })),
        "temperature": 0,
        "top_k": 0,
        "top_p": 1,
        "min_p": 0,
        "repetition_penalty": 1,
        "presence_penalty": 0,
        "frequency_penalty": 0,
        "seed": 0,
        "return_token_ids": true,
    })
    .to_string()
}

fn assert_non_streaming_response(response: &str) -> Result<(), Box<dyn std::error::Error>> {
    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "unexpected response: {response}"
    );
    let body = response_json(response)?;
    assert_eq!(body["object"], "chat.completion");
    assert_eq!(body["model"], REAL_MODEL_ID);
    assert_eq!(body["choices"][0]["message"]["content"], EXPECTED_CONTENT);
    assert_eq!(body["choices"][0]["finish_reason"], "stop");
    assert_eq!(body["usage"]["prompt_tokens"], 10);
    assert_eq!(body["usage"]["completion_tokens"], 3);
    assert_eq!(body["usage"]["total_tokens"], 13);
    assert_eq!(
        body["usage"]["completion_tokens_details"]["ferrite_finish_source"],
        "eos"
    );
    assert!(
        !body["choices"][0]["message"]["content"]
            .as_str()
            .is_some_and(|content| content.contains("<|end|>")),
        "terminal Phi-3 token became visible: {body}"
    );
    Ok(())
}

fn assert_streaming_response(response: &str) -> Result<(), Box<dyn std::error::Error>> {
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
        "missing stream terminator"
    );

    let events = sse_json_events(response)?;
    let content = events
        .iter()
        .filter_map(|event| event["choices"][0]["delta"]["content"].as_str())
        .collect::<String>();
    let token_ids = events
        .iter()
        .filter_map(|event| event["choices"][0]["token_ids"].as_array())
        .flatten()
        .map(|token_id| {
            token_id
                .as_u64()
                .ok_or_else(|| format!("non-integer streaming token ID: {token_id}").into())
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    let finish_events = events
        .iter()
        .filter(|event| event["choices"][0]["finish_reason"] == "stop")
        .count();
    let usage = events
        .iter()
        .find_map(|event| event.get("usage").filter(|usage| usage.is_object()))
        .ok_or("missing streaming usage event")?;

    assert_eq!(content, EXPECTED_CONTENT);
    assert_eq!(token_ids, EXPECTED_VISIBLE_TOKEN_IDS);
    assert_eq!(finish_events, 1);
    assert_eq!(usage["prompt_tokens"], 10);
    assert_eq!(usage["completion_tokens"], 3);
    assert_eq!(usage["total_tokens"], 13);
    assert_eq!(
        usage["completion_tokens_details"]["ferrite_finish_source"],
        "eos"
    );
    assert!(!content.contains("<|end|>"));
    Ok(())
}

fn verified_phi3_model_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let model_path = std::env::var_os("FERRITE_PHI3_MODEL")
        .map(PathBuf::from)
        .ok_or("FERRITE_PHI3_MODEL is not set")?;
    let metadata = model_path.metadata().map_err(|error| {
        format!(
            "failed to inspect Phi-3 model artifact {}: {error}",
            model_path.display()
        )
    })?;
    if !metadata.is_file() || metadata.len() != EXPECTED_MODEL_BYTES {
        return Err(format!(
            "unexpected Phi-3 model artifact {}: file={}, bytes={}, expected_bytes={EXPECTED_MODEL_BYTES}",
            model_path.display(),
            metadata.is_file(),
            metadata.len()
        )
        .into());
    }
    let actual_sha256 = sha256_file(&model_path)?;
    if actual_sha256 != EXPECTED_MODEL_SHA256 {
        return Err(format!(
            "unexpected Phi-3 model SHA-256 for {}: {actual_sha256}",
            model_path.display()
        )
        .into());
    }
    Ok(model_path)
}

fn sha256_file(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
