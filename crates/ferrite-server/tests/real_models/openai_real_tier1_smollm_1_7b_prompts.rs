use crate::support;

use std::path::PathBuf;

use support::http::{response_json, send_http_request};

const DEFAULT_MODEL_PATH: &str = "target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf";
const REAL_MODEL_ID: &str = "smollm2-1.7b-q4_k_m";

#[tokio::test]
#[ignore = "requires local SmolLM2-1.7B Q4_K_M GGUF model artifact"]
async fn live_http_server_matches_smollm_1_7b_q4_first_tokens_for_reference_prompts()
-> Result<(), Box<dyn std::error::Error>> {
    let model_path = smollm_1_7b_q4_model_path()?;
    let server = support::LiveServer::start_with_existing_model(REAL_MODEL_ID, model_path).await?;
    let cases = [
        PromptCase {
            prompt: "hello world",
            prompt_tokens: 2,
            text: "\"",
        },
        PromptCase {
            prompt: "The capital of France is",
            prompt_tokens: 5,
            text: " Paris",
        },
        PromptCase {
            prompt: "Once upon a time",
            prompt_tokens: 4,
            text: ",",
        },
        PromptCase {
            prompt: "Rust is a systems programming language",
            prompt_tokens: 7,
            text: " that",
        },
        PromptCase {
            prompt: "Machine learning models can",
            prompt_tokens: 4,
            text: " be",
        },
        PromptCase {
            prompt: "The recipe calls for",
            prompt_tokens: 4,
            text: " ",
        },
    ];

    for case in cases {
        let response = send_http_request(
            server.addr(),
            "POST",
            "/v1/completions",
            completion_body(case.prompt).as_bytes(),
        )
        .await?;
        assert_smollm_completion_response(&response, case)?;
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct PromptCase {
    prompt: &'static str,
    prompt_tokens: u64,
    text: &'static str,
}

fn completion_body(prompt: &str) -> String {
    serde_json::json!({
        "model": REAL_MODEL_ID,
        "prompt": prompt,
        "max_tokens": 1,
    })
    .to_string()
}

fn assert_smollm_completion_response(
    response: &str,
    case: PromptCase,
) -> Result<(), Box<dyn std::error::Error>> {
    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "unexpected response for prompt {:?}: {response}",
        case.prompt
    );
    let body = response_json(response)?;
    assert_eq!(body["object"], "text_completion");
    assert_eq!(body["model"], REAL_MODEL_ID);
    assert_eq!(body["choices"][0]["text"], case.text);
    assert_eq!(body["usage"]["prompt_tokens"], case.prompt_tokens);
    assert_eq!(body["usage"]["completion_tokens"], 1);
    assert_eq!(body["usage"]["total_tokens"], case.prompt_tokens + 1);
    Ok(())
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
