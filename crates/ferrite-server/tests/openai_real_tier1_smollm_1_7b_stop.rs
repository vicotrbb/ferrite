mod support;

use std::path::PathBuf;

use support::http::send_http_request;
use support::stop_sequences::{
    assert_stop_chat_response, assert_stop_chat_stream, assert_stop_completion_response,
    assert_stop_completion_stream, StopSequenceExpectation,
};

const DEFAULT_MODEL_PATH: &str = "target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf";
const REAL_MODEL_ID: &str = "smollm2-1.7b-q4_k_m";

#[tokio::test]
#[ignore = "requires local SmolLM2-1.7B Q4_K_M GGUF model artifact"]
async fn live_http_server_applies_stop_sequences_with_smollm_1_7b_q4_model(
) -> Result<(), Box<dyn std::error::Error>> {
    let model_path = smollm_1_7b_q4_model_path()?;
    let server = support::LiveServer::start_with_existing_model(REAL_MODEL_ID, model_path).await?;

    let completion_body = completion_stop_body(false);
    let completion_response = send_http_request(
        server.addr(),
        "POST",
        "/v1/completions",
        completion_body.as_bytes(),
    )
    .await?;
    assert_stop_completion_response(&completion_response, smollm_1_7b_q4_stop_expectation())?;

    let completion_stream_body = completion_stop_body(true);
    let completion_stream_response = send_http_request(
        server.addr(),
        "POST",
        "/v1/completions",
        completion_stream_body.as_bytes(),
    )
    .await?;
    assert_stop_completion_stream(&completion_stream_response)?;

    let chat_body = chat_stop_body(false);
    let chat_response = send_http_request(
        server.addr(),
        "POST",
        "/v1/chat/completions",
        chat_body.as_bytes(),
    )
    .await?;
    assert_stop_chat_response(&chat_response, smollm_1_7b_q4_stop_expectation())?;

    let chat_stream_body = chat_stop_body(true);
    let chat_stream_response = send_http_request(
        server.addr(),
        "POST",
        "/v1/chat/completions",
        chat_stream_body.as_bytes(),
    )
    .await?;
    assert_stop_chat_stream(&chat_stream_response)?;
    Ok(())
}

fn smollm_1_7b_q4_stop_expectation() -> StopSequenceExpectation {
    StopSequenceExpectation {
        model_id: REAL_MODEL_ID,
        completion_prompt_tokens: 2,
        chat_prompt_tokens: 9,
    }
}

fn completion_stop_body(stream: bool) -> String {
    serde_json::json!({
        "model": REAL_MODEL_ID,
        "prompt": "hello world",
        "max_tokens": 1,
        "stream": stream,
        "stop": "\"",
    })
    .to_string()
}

fn chat_stop_body(stream: bool) -> String {
    serde_json::json!({
        "model": REAL_MODEL_ID,
        "messages": [
            {
                "role": "user",
                "content": "hello world",
            },
        ],
        "max_completion_tokens": 1,
        "stream": stream,
        "stop": "1",
    })
    .to_string()
}

fn smollm_1_7b_q4_model_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let model_path = std::env::var_os("FERRITE_SMOLLM_1_7B_Q4_MODEL")
        .map(PathBuf::from)
        .unwrap_or_else(default_model_path);
    if !model_path.is_file() {
        return Err(format!(
            "missing SmolLM2-1.7B Q4_K_M model artifact: {}",
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
