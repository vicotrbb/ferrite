use crate::support;

use std::path::PathBuf;

use support::http::{send_http_request, sse_json_events};

const DEFAULT_MODEL_PATH: &str = "target/models/qwen2.5-1.5b-instruct-q8_0.gguf";
const REAL_MODEL_ID: &str = "qwen2.5-1.5b-q8_0";

#[tokio::test]
#[ignore = "requires local Qwen2.5-1.5B Q8_0 GGUF model artifact"]
async fn live_http_server_streams_qwen_1_5b_q8_first_tokens_for_reference_prompts()
-> Result<(), Box<dyn std::error::Error>> {
    let model_path = qwen_1_5b_q8_model_path()?;
    let server = support::LiveServer::start_with_existing_model(REAL_MODEL_ID, model_path).await?;

    for case in prompt_cases() {
        let response = send_http_request(
            server.addr(),
            "POST",
            "/v1/completions",
            streaming_completion_body(case.prompt).as_bytes(),
        )
        .await?;
        assert_qwen_stream_response(&response, case)?;
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct PromptCase {
    prompt: &'static str,
    text: &'static str,
}

fn prompt_cases() -> [PromptCase; 6] {
    [
        PromptCase {
            prompt: "hello world",
            text: "\n",
        },
        PromptCase {
            prompt: "The capital of France is",
            text: " Paris",
        },
        PromptCase {
            prompt: "Once upon a time",
            text: ",",
        },
        PromptCase {
            prompt: "Rust is a systems programming language",
            text: " that",
        },
        PromptCase {
            prompt: "Machine learning models can",
            text: " be",
        },
        PromptCase {
            prompt: "The recipe calls for",
            text: " ",
        },
    ]
}

fn streaming_completion_body(prompt: &str) -> String {
    serde_json::json!({
        "model": REAL_MODEL_ID,
        "prompt": prompt,
        "max_tokens": 1,
        "stream": true,
    })
    .to_string()
}

fn assert_qwen_stream_response(
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
        assert_eq!(event["object"], "text_completion");
        assert_eq!(event["model"], REAL_MODEL_ID);
    }
    let generated_texts = events
        .iter()
        .filter_map(|event| {
            let choice = &event["choices"][0];
            (choice["finish_reason"].is_null()).then(|| choice["text"].as_str())?
        })
        .collect::<Vec<_>>();
    assert_eq!(
        generated_texts,
        vec![case.text],
        "unexpected generated stream text chunks"
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
    assert_eq!(stop_events[0]["choices"][0]["text"], "");
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
