mod support;

use std::path::PathBuf;

use serde_json::Value;
use support::http::{response_json, send_http_request};

const DEFAULT_MODEL_PATH: &str = "target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf";
const REAL_MODEL_ID: &str = "smollm2-1.7b-q4_k_m";

#[tokio::test]
#[ignore = "requires local SmolLM2-1.7B Q4_K_M GGUF model artifact"]
async fn live_http_server_chats_with_smollm_1_7b_q4_reference_prompt(
) -> Result<(), Box<dyn std::error::Error>> {
    let model_path = smollm_1_7b_q4_model_path()?;
    let server = support::LiveServer::start_with_existing_model(REAL_MODEL_ID, model_path).await?;
    let case = PromptCase {
        prompt: "hello world",
        prompt_tokens: 9,
        content: "1",
    };

    let response = send_http_request(
        server.addr(),
        "POST",
        "/v1/chat/completions",
        chat_body(case.prompt, false).as_bytes(),
    )
    .await?;
    assert_smollm_chat_response(&response, case)?;

    let stream_response = send_http_request(
        server.addr(),
        "POST",
        "/v1/chat/completions",
        chat_body(case.prompt, true).as_bytes(),
    )
    .await?;
    assert_smollm_chat_stream_response(&stream_response, case)?;
    Ok(())
}

#[tokio::test]
#[ignore = "requires local SmolLM2-1.7B Q4_K_M GGUF model artifact"]
async fn live_http_server_matches_smollm_1_7b_q4_chat_first_tokens_for_reference_prompts(
) -> Result<(), Box<dyn std::error::Error>> {
    let model_path = smollm_1_7b_q4_model_path()?;
    let server = support::LiveServer::start_with_existing_model(REAL_MODEL_ID, model_path).await?;

    for case in prompt_cases() {
        let response = send_http_request(
            server.addr(),
            "POST",
            "/v1/chat/completions",
            chat_body(case.prompt, false).as_bytes(),
        )
        .await?;
        assert_smollm_chat_response(&response, case)?;
    }
    Ok(())
}

#[tokio::test]
#[ignore = "requires local SmolLM2-1.7B Q4_K_M GGUF model artifact"]
async fn live_http_server_streams_smollm_1_7b_q4_chat_first_tokens_for_reference_prompts(
) -> Result<(), Box<dyn std::error::Error>> {
    let model_path = smollm_1_7b_q4_model_path()?;
    let server = support::LiveServer::start_with_existing_model(REAL_MODEL_ID, model_path).await?;

    for case in prompt_cases() {
        let response = send_http_request(
            server.addr(),
            "POST",
            "/v1/chat/completions",
            chat_body(case.prompt, true).as_bytes(),
        )
        .await?;
        assert_smollm_chat_stream_response(&response, case)?;
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct PromptCase {
    prompt: &'static str,
    prompt_tokens: u64,
    content: &'static str,
}

fn prompt_cases() -> [PromptCase; 6] {
    [
        PromptCase {
            prompt: "hello world",
            prompt_tokens: 9,
            content: "1",
        },
        PromptCase {
            prompt: "The capital of France is",
            prompt_tokens: 12,
            content: "\n",
        },
        PromptCase {
            prompt: "Once upon a time",
            prompt_tokens: 11,
            content: "\n",
        },
        PromptCase {
            prompt: "Rust is a systems programming language",
            prompt_tokens: 13,
            content: "\n",
        },
        PromptCase {
            prompt: "Machine learning models can",
            prompt_tokens: 11,
            content: "1",
        },
        PromptCase {
            prompt: "The recipe calls for",
            prompt_tokens: 11,
            content: "1",
        },
    ]
}

fn chat_body(prompt: &str, stream: bool) -> String {
    serde_json::json!({
        "model": REAL_MODEL_ID,
        "messages": [
            {
                "role": "user",
                "content": prompt,
            },
        ],
        "max_completion_tokens": 1,
        "stream": stream,
    })
    .to_string()
}

fn assert_smollm_chat_response(
    response: &str,
    case: PromptCase,
) -> Result<(), Box<dyn std::error::Error>> {
    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "unexpected response for prompt {:?}: {response}",
        case.prompt
    );
    let body = response_json(response)?;
    assert_eq!(body["object"], "chat.completion");
    assert_eq!(body["model"], REAL_MODEL_ID);
    assert_eq!(body["choices"][0]["message"]["content"], case.content);
    assert_eq!(body["usage"]["prompt_tokens"], case.prompt_tokens);
    assert_eq!(body["usage"]["completion_tokens"], 1);
    assert_eq!(body["usage"]["total_tokens"], case.prompt_tokens + 1);
    Ok(())
}

fn assert_smollm_chat_stream_response(
    response: &str,
    case: PromptCase,
) -> Result<(), Box<dyn std::error::Error>> {
    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "unexpected response for prompt {:?}: {response}",
        case.prompt
    );
    assert!(
        response
            .to_ascii_lowercase()
            .contains("content-type: text/event-stream"),
        "unexpected response headers for prompt {:?}: {response}",
        case.prompt
    );
    assert!(
        response.contains("data: [DONE]"),
        "missing stream terminator for prompt {:?}: {response}",
        case.prompt
    );

    let events = sse_json_events(response)?;
    assert!(
        !events.is_empty(),
        "missing JSON SSE events for prompt {:?}: {response}",
        case.prompt
    );
    for event in &events {
        assert_eq!(event["object"], "chat.completion.chunk");
        assert_eq!(event["model"], REAL_MODEL_ID);
    }
    let generated_content = events
        .iter()
        .filter_map(|event| {
            let choice = &event["choices"][0];
            (choice["finish_reason"].is_null()).then(|| choice["delta"]["content"].as_str())?
        })
        .collect::<Vec<_>>();
    assert_eq!(
        generated_content,
        vec![case.content],
        "unexpected generated chat stream chunks"
    );
    let stop_events = events
        .iter()
        .filter(|event| event["choices"][0]["finish_reason"] == "stop")
        .collect::<Vec<_>>();
    assert_eq!(
        stop_events.len(),
        1,
        "expected exactly one terminal stream chunk"
    );
    assert!(
        stop_events[0]["choices"][0]["delta"]["content"].is_null(),
        "terminal chat stream chunk should not include content"
    );
    Ok(())
}

fn sse_json_events(response: &str) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    let (_, body) = response
        .split_once("\r\n\r\n")
        .ok_or("expected HTTP response body")?;
    body.lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .filter(|data| *data != "[DONE]")
        .map(serde_json::from_str)
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
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
