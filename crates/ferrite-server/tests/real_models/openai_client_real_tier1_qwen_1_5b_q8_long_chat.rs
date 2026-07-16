use crate::support;

use async_openai::types::chat::{
    ChatCompletionRequestMessage, ChatCompletionRequestUserMessage, ChatCompletionStreamOptions,
    CreateChatCompletionRequest, FinishReason,
};
use std::path::PathBuf;
use support::openai_client::ferrite_client;
use tokio_stream::StreamExt;

const DEFAULT_MODEL_PATH: &str = "target/models/qwen2.5-1.5b-instruct-q8_0.gguf";
const REAL_MODEL_ID: &str = "qwen2.5-1.5b-q8_0";
const EXPECTED_CHAT_CONTENT: &str = "Hello! How can I help you today?";

#[tokio::test]
#[ignore = "requires local Qwen2.5-1.5B Q8_0 GGUF model artifact"]
async fn async_openai_client_chats_with_32_token_limit_and_qwen_1_5b_q8_model()
-> Result<(), Box<dyn std::error::Error>> {
    let server =
        support::LiveServer::start_with_existing_model(REAL_MODEL_ID, qwen_1_5b_q8_model_path()?)
            .await?;
    let client = ferrite_client(&server, "local-test");

    let response = client
        .chat()
        .create(chat_request(REAL_MODEL_ID, false))
        .await?;

    assert_eq!(response.object, "chat.completion");
    assert_eq!(response.model, REAL_MODEL_ID);
    assert_eq!(response.choices[0].finish_reason, Some(FinishReason::Stop));
    assert_eq!(
        response.choices[0].message.content.as_deref(),
        Some(EXPECTED_CHAT_CONTENT),
        "unexpected deterministic assistant content: {response:?}"
    );
    assert_eq!(
        response.usage.as_ref().map(|usage| usage.completion_tokens),
        Some(10)
    );

    let mut stream = client
        .chat()
        .create_stream(chat_request(REAL_MODEL_ID, true))
        .await?;

    let mut streamed_content = String::new();
    let mut completion_tokens = None;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        for choice in chunk.choices {
            if let Some(delta) = choice.delta.content {
                streamed_content.push_str(&delta);
            }
        }
        if let Some(usage) = chunk.usage {
            completion_tokens = Some(usage.completion_tokens);
        }
    }

    assert_eq!(streamed_content, EXPECTED_CHAT_CONTENT);
    assert_eq!(completion_tokens, Some(10));
    Ok(())
}

fn chat_request(model_id: &str, stream: bool) -> CreateChatCompletionRequest {
    CreateChatCompletionRequest {
        model: model_id.to_owned(),
        messages: vec![ChatCompletionRequestMessage::User(
            ChatCompletionRequestUserMessage {
                content: "hello world".into(),
                ..Default::default()
            },
        )],
        max_completion_tokens: Some(32),
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
