use crate::support;

use async_openai::types::{
    chat::{ChatCompletionStreamOptions, CompletionFinishReason, Prompt},
    completions::CreateCompletionRequest,
};
use std::path::PathBuf;
use support::openai_client::ferrite_client;
use tokio_stream::StreamExt;

const DEFAULT_MODEL_PATH: &str = "target/models/qwen2.5-1.5b-instruct-q8_0.gguf";
const REAL_MODEL_ID: &str = "qwen2.5-1.5b-q8_0";

#[tokio::test]
#[ignore = "requires local Qwen2.5-1.5B Q8_0 GGUF model artifact"]
async fn async_openai_client_completes_32_tokens_with_qwen_1_5b_q8_model()
-> Result<(), Box<dyn std::error::Error>> {
    let server =
        support::LiveServer::start_with_existing_model(REAL_MODEL_ID, qwen_1_5b_q8_model_path()?)
            .await?;
    let client = ferrite_client(&server, "local-test");

    let response = client
        .completions()
        .create(completion_request(false))
        .await?;

    assert_eq!(response.object, "text_completion");
    assert_eq!(response.model, REAL_MODEL_ID);
    assert_eq!(
        response.choices[0].finish_reason,
        Some(CompletionFinishReason::Length)
    );
    assert!(
        !response.choices[0].text.is_empty(),
        "expected non-empty completion text: {response:?}"
    );
    assert_eq!(
        response.usage.as_ref().map(|usage| usage.completion_tokens),
        Some(32)
    );

    let mut stream = client
        .completions()
        .create_stream(completion_request(true))
        .await?;

    let mut text = String::new();
    let mut completion_tokens = None;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        for choice in chunk.choices {
            text.push_str(&choice.text);
        }
        if let Some(usage) = chunk.usage {
            completion_tokens = Some(usage.completion_tokens);
        }
    }

    assert!(!text.is_empty(), "expected non-empty streamed text");
    assert_eq!(completion_tokens, Some(32));
    Ok(())
}

fn completion_request(stream: bool) -> CreateCompletionRequest {
    CreateCompletionRequest {
        model: REAL_MODEL_ID.to_owned(),
        prompt: Prompt::String("hello world".to_owned()),
        max_tokens: Some(32),
        stream: Some(stream),
        stream_options: stream.then_some(ChatCompletionStreamOptions {
            include_usage: Some(true),
            include_obfuscation: None,
        }),
        ..Default::default()
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
