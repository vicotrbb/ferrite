mod support;

use std::path::PathBuf;

use support::http::send_http_request;
use support::stop_sequences::{
    assert_stop_completion_response, assert_stop_completion_stream, StopSequenceExpectation,
};

const DEFAULT_MODEL_PATH: &str = "target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf";
const REAL_MODEL_ID: &str = "smollm2-1.7b-q4_k_m";

#[tokio::test]
#[ignore = "requires local SmolLM2-1.7B Q4_K_M GGUF model artifact"]
async fn live_http_server_applies_completion_stop_sequences_to_smollm_1_7b_q4_reference_prompts(
) -> Result<(), Box<dyn std::error::Error>> {
    let model_path = smollm_1_7b_q4_model_path()?;
    let server = support::LiveServer::start_with_existing_model(REAL_MODEL_ID, model_path).await?;

    for case in completion_stop_cases() {
        let completion_body = completion_stop_case_body(case, false);
        let completion_response = send_http_request(
            server.addr(),
            "POST",
            "/v1/completions",
            completion_body.as_bytes(),
        )
        .await?;
        assert_stop_completion_response(
            &completion_response,
            StopSequenceExpectation {
                model_id: REAL_MODEL_ID,
                completion_prompt_tokens: case.prompt_tokens,
                chat_prompt_tokens: 0,
            },
        )?;

        let completion_stream_body = completion_stop_case_body(case, true);
        let completion_stream_response = send_http_request(
            server.addr(),
            "POST",
            "/v1/completions",
            completion_stream_body.as_bytes(),
        )
        .await?;
        assert_stop_completion_stream(&completion_stream_response)?;
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct CompletionStopCase {
    prompt: &'static str,
    prompt_tokens: u64,
    stop: &'static str,
}

fn completion_stop_cases() -> [CompletionStopCase; 6] {
    [
        CompletionStopCase {
            prompt: "hello world",
            prompt_tokens: 2,
            stop: "\"",
        },
        CompletionStopCase {
            prompt: "The capital of France is",
            prompt_tokens: 5,
            stop: " Paris",
        },
        CompletionStopCase {
            prompt: "Once upon a time",
            prompt_tokens: 4,
            stop: ",",
        },
        CompletionStopCase {
            prompt: "Rust is a systems programming language",
            prompt_tokens: 7,
            stop: " that",
        },
        CompletionStopCase {
            prompt: "Machine learning models can",
            prompt_tokens: 4,
            stop: " also",
        },
        CompletionStopCase {
            prompt: "The recipe calls for",
            prompt_tokens: 4,
            stop: " ",
        },
    ]
}

fn completion_stop_case_body(case: CompletionStopCase, stream: bool) -> String {
    serde_json::json!({
        "model": REAL_MODEL_ID,
        "prompt": case.prompt,
        "max_tokens": 1,
        "stream": stream,
        "stop": case.stop,
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
