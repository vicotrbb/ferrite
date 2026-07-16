use crate::support;

use std::path::PathBuf;

use support::http::send_http_request;
use support::stop_sequences::{
    StopSequenceExpectation, assert_stop_chat_response, assert_stop_chat_stream,
    assert_stop_completion_response, assert_stop_completion_stream,
};

const DEFAULT_MODEL_PATH: &str = "target/models/qwen2.5-1.5b-instruct-q8_0.gguf";
const REAL_MODEL_ID: &str = "qwen2.5-1.5b-q8_0";

#[tokio::test]
#[ignore = "requires local Qwen2.5-1.5B Q8_0 GGUF model artifact"]
async fn live_http_server_applies_stop_sequences_with_qwen_1_5b_q8_model()
-> Result<(), Box<dyn std::error::Error>> {
    let model_path = qwen_1_5b_q8_model_path()?;
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
    assert_stop_completion_response(&completion_response, qwen_1_5b_q8_stop_expectation())?;

    let completion_stream_body = format!(
        r#"{{"model":"{REAL_MODEL_ID}","prompt":"hello world","max_tokens":1,"stream":true,"stop":"\n"}}"#
    );
    let completion_stream_response = send_http_request(
        server.addr(),
        "POST",
        "/v1/completions",
        completion_stream_body.as_bytes(),
    )
    .await?;
    assert_stop_completion_stream(&completion_stream_response)?;

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
    assert_stop_chat_response(&chat_response, qwen_1_5b_q8_stop_expectation())?;

    let chat_stream_body = format!(
        r#"{{"model":"{REAL_MODEL_ID}","messages":[{{"role":"user","content":"hello world"}}],"max_completion_tokens":1,"stream":true,"stop":"Hello"}}"#
    );
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

fn qwen_1_5b_q8_stop_expectation() -> StopSequenceExpectation {
    StopSequenceExpectation {
        model_id: REAL_MODEL_ID,
        completion_prompt_tokens: 2,
        chat_prompt_tokens: 31,
    }
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
