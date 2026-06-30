mod support;

use async_openai::types::chat::Prompt;
use async_openai::types::{
    chat::{
        ChatCompletionRequestMessage, ChatCompletionRequestUserMessage,
        ChatCompletionStreamOptions, CreateChatCompletionRequest,
    },
    completions::CreateCompletionRequest,
};
use std::path::PathBuf;
use support::openai_client::ferrite_client;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;

const DEFAULT_MODEL_PATH: &str = "target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf";
const REAL_MODEL_ID: &str = "qwen2.5-0.5b";
static REAL_MODEL_TEST_LOCK: Mutex<()> = Mutex::const_new(());

#[tokio::test]
#[ignore = "requires local Tier 1 GGUF model artifact"]
async fn async_openai_client_generates_with_real_tier1_model(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_lock = REAL_MODEL_TEST_LOCK.lock().await;
    let server =
        support::LiveServer::start_with_existing_model(REAL_MODEL_ID, real_model_path()?).await?;
    let client = ferrite_client(&server, "local-test");

    let response = client
        .completions()
        .create(CreateCompletionRequest {
            model: REAL_MODEL_ID.to_owned(),
            prompt: Prompt::String("hello world".to_owned()),
            max_tokens: Some(1),
            ..Default::default()
        })
        .await?;

    assert_eq!(response.object, "text_completion");
    assert_eq!(response.model, REAL_MODEL_ID);
    assert_eq!(response.choices[0].text, "\n");
    assert_eq!(
        response.usage.as_ref().map(|usage| usage.completion_tokens),
        Some(1)
    );
    Ok(())
}

#[tokio::test]
#[ignore = "requires local Tier 1 GGUF model artifact"]
async fn async_openai_client_streams_with_real_tier1_model(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_lock = REAL_MODEL_TEST_LOCK.lock().await;
    let server =
        support::LiveServer::start_with_existing_model(REAL_MODEL_ID, real_model_path()?).await?;
    let client = ferrite_client(&server, "local-test");

    let mut stream = client
        .completions()
        .create_stream(CreateCompletionRequest {
            model: REAL_MODEL_ID.to_owned(),
            prompt: Prompt::String("hello world".to_owned()),
            max_tokens: Some(1),
            stream_options: Some(ChatCompletionStreamOptions {
                include_usage: Some(true),
                include_obfuscation: None,
            }),
            ..Default::default()
        })
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

    assert_eq!(text, "\n");
    assert_eq!(completion_tokens, Some(1));
    Ok(())
}

#[tokio::test]
#[ignore = "requires local Tier 1 GGUF model artifact"]
async fn async_openai_client_chats_with_real_tier1_model() -> Result<(), Box<dyn std::error::Error>>
{
    let _test_lock = REAL_MODEL_TEST_LOCK.lock().await;
    let server =
        support::LiveServer::start_with_existing_model(REAL_MODEL_ID, real_model_path()?).await?;
    let client = ferrite_client(&server, "local-test");

    let response = client.chat().create(chat_request()).await?;

    assert_eq!(response.object, "chat.completion");
    assert_eq!(response.model, REAL_MODEL_ID);
    assert_eq!(response.choices[0].message.content.as_deref(), Some("你好"));
    assert_eq!(
        response.usage.as_ref().map(|usage| usage.completion_tokens),
        Some(1)
    );
    Ok(())
}

#[tokio::test]
#[ignore = "requires local Tier 1 GGUF model artifact"]
async fn async_openai_client_streams_chat_with_real_tier1_model(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_lock = REAL_MODEL_TEST_LOCK.lock().await;
    let server =
        support::LiveServer::start_with_existing_model(REAL_MODEL_ID, real_model_path()?).await?;
    let client = ferrite_client(&server, "local-test");
    let mut request = chat_request();
    request.stream_options = Some(ChatCompletionStreamOptions {
        include_usage: Some(true),
        include_obfuscation: None,
    });

    let mut stream = client.chat().create_stream(request).await?;

    let mut content = String::new();
    let mut completion_tokens = None;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        for choice in chunk.choices {
            if let Some(delta) = choice.delta.content {
                content.push_str(&delta);
            }
        }
        if let Some(usage) = chunk.usage {
            completion_tokens = Some(usage.completion_tokens);
        }
    }

    assert_eq!(content, "你好");
    assert_eq!(completion_tokens, Some(1));
    Ok(())
}

fn chat_request() -> CreateChatCompletionRequest {
    CreateChatCompletionRequest {
        model: REAL_MODEL_ID.to_owned(),
        messages: vec![ChatCompletionRequestMessage::User(
            ChatCompletionRequestUserMessage {
                content: "hello world".into(),
                ..Default::default()
            },
        )],
        max_completion_tokens: Some(1),
        ..Default::default()
    }
}

fn real_model_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let model_path = std::env::var_os("FERRITE_REAL_TIER1_MODEL")
        .map(PathBuf::from)
        .unwrap_or_else(default_model_path);
    if !model_path.is_file() {
        return Err(format!(
            "missing real Tier 1 model artifact: {}",
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
