use crate::support;

use std::path::PathBuf;

use support::http::{response_json, send_http_request, sse_json_events};

const DEFAULT_MODEL_PATH: &str = "target/models/qwen2.5-1.5b-instruct-q8_0.gguf";
const REAL_MODEL_ID: &str = "qwen2.5-1.5b-q8_0";

#[tokio::test]
#[ignore = "requires local Qwen2.5-1.5B Q8_0 GGUF model artifact"]
async fn live_http_server_matches_qwen_1_5b_q8_first_tokens_for_reference_prompts()
-> Result<(), Box<dyn std::error::Error>> {
    let model_path = qwen_1_5b_q8_model_path()?;
    let server = support::LiveServer::start_with_existing_model(REAL_MODEL_ID, model_path).await?;

    for case in prompt_cases() {
        let response = send_http_request(
            server.addr(),
            "POST",
            "/v1/completions",
            completion_body(case.prompt).as_bytes(),
        )
        .await?;
        assert_qwen_completion_response(&response, case)?;
    }
    Ok(())
}

#[tokio::test]
#[ignore = "requires local Qwen2.5-1.5B Q8_0 GGUF model artifact"]
async fn live_http_server_matches_qwen_1_5b_q8_chat_first_tokens_for_reference_prompts()
-> Result<(), Box<dyn std::error::Error>> {
    let model_path = qwen_1_5b_q8_model_path()?;
    let server = support::LiveServer::start_with_existing_model(REAL_MODEL_ID, model_path).await?;

    for case in prompt_cases() {
        let response = send_http_request(
            server.addr(),
            "POST",
            "/v1/chat/completions",
            chat_body(case.prompt, false).as_bytes(),
        )
        .await?;
        assert_qwen_chat_response(&response, case)?;
    }
    Ok(())
}

#[tokio::test]
#[ignore = "requires local Qwen2.5-1.5B Q8_0 GGUF model artifact"]
async fn live_http_server_streams_qwen_1_5b_q8_chat_first_tokens_for_reference_prompts()
-> Result<(), Box<dyn std::error::Error>> {
    let model_path = qwen_1_5b_q8_model_path()?;
    let server = support::LiveServer::start_with_existing_model(REAL_MODEL_ID, model_path).await?;

    for case in prompt_cases() {
        let response = send_http_request(
            server.addr(),
            "POST",
            "/v1/chat/completions",
            chat_body(case.prompt, true).as_bytes(),
        )
        .await?;
        assert_qwen_chat_stream_response(&response, case)?;
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct PromptCase {
    prompt: &'static str,
    completion_prompt_tokens: u64,
    completion_text: &'static str,
    chat_prompt_tokens: u64,
    chat_content: &'static str,
    chat_stream_content: &'static str,
}

fn prompt_cases() -> [PromptCase; 6] {
    [
        PromptCase {
            prompt: "hello world",
            completion_prompt_tokens: 2,
            completion_text: "\n",
            chat_prompt_tokens: 31,
            chat_content: "Hello",
            chat_stream_content: "Hello",
        },
        PromptCase {
            prompt: "The capital of France is",
            completion_prompt_tokens: 5,
            completion_text: " Paris",
            chat_prompt_tokens: 34,
            chat_content: "The",
            chat_stream_content: "The",
        },
        PromptCase {
            prompt: "Once upon a time",
            completion_prompt_tokens: 4,
            completion_text: ",",
            chat_prompt_tokens: 33,
            chat_content: "I",
            chat_stream_content: "I",
        },
        PromptCase {
            prompt: "Rust is a systems programming language",
            completion_prompt_tokens: 7,
            completion_text: " that",
            chat_prompt_tokens: 36,
            chat_content: "R",
            chat_stream_content: "R",
        },
        PromptCase {
            prompt: "Machine learning models can",
            completion_prompt_tokens: 4,
            completion_text: " be",
            chat_prompt_tokens: 33,
            chat_content: "Machine",
            chat_stream_content: "Machine",
        },
        PromptCase {
            prompt: "The recipe calls for",
            completion_prompt_tokens: 4,
            completion_text: " ",
            chat_prompt_tokens: 33,
            chat_content: "I",
            chat_stream_content: "I",
        },
    ]
}

fn completion_body(prompt: &str) -> String {
    serde_json::json!({
        "model": REAL_MODEL_ID,
        "prompt": prompt,
        "max_tokens": 1,
    })
    .to_string()
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

fn assert_qwen_completion_response(
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
    assert_eq!(body["choices"][0]["text"], case.completion_text);
    assert_eq!(
        body["usage"]["prompt_tokens"],
        case.completion_prompt_tokens
    );
    assert_eq!(body["usage"]["completion_tokens"], 1);
    assert_eq!(
        body["usage"]["total_tokens"],
        case.completion_prompt_tokens + 1
    );
    Ok(())
}

fn assert_qwen_chat_response(
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
    assert_eq!(body["choices"][0]["message"]["content"], case.chat_content);
    assert_eq!(body["usage"]["prompt_tokens"], case.chat_prompt_tokens);
    assert_eq!(body["usage"]["completion_tokens"], 1);
    assert_eq!(body["usage"]["total_tokens"], case.chat_prompt_tokens + 1);
    Ok(())
}

fn assert_qwen_chat_stream_response(
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
            (choice["finish_reason"].is_null() && choice["delta"]["role"].is_null())
                .then(|| choice["delta"]["content"].as_str())?
        })
        .collect::<Vec<_>>();
    assert_eq!(
        generated_content,
        vec![case.chat_stream_content],
        "unexpected generated chat stream chunks"
    );
    let stop_events = events
        .iter()
        .filter(|event| event["choices"][0]["finish_reason"] == "length")
        .collect::<Vec<_>>();
    assert_eq!(
        stop_events.len(),
        1,
        "expected exactly one length terminal stream chunk"
    );
    assert!(
        stop_events[0]["choices"][0]["delta"]["content"].is_null(),
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
