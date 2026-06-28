mod support;

use async_openai::{
    config::OpenAIConfig,
    types::chat::{
        ChatCompletionRequestMessage, ChatCompletionRequestUserMessage,
        ChatCompletionStreamOptions, CreateChatCompletionRequest, Prompt,
    },
    types::completions::CreateCompletionRequest,
    Client,
};
use tokio_stream::StreamExt;

#[tokio::test]
async fn async_openai_client_lists_ferrite_model() -> Result<(), Box<dyn std::error::Error>> {
    let server = support::LiveServer::start().await?;
    let config = OpenAIConfig::new()
        .with_api_base(format!("http://{}/v1", server.addr()))
        .with_api_key("local-test");
    let client = Client::with_config(config);

    let response = client.models().list().await?;

    assert_eq!(response.object, "list");
    assert_eq!(response.data.len(), 1);
    assert_eq!(response.data[0].id, support::MODEL_ID);
    assert_eq!(response.data[0].object, "model");
    assert_eq!(response.data[0].owned_by, "ferrite");
    Ok(())
}

#[tokio::test]
async fn async_openai_client_retrieves_ferrite_model() -> Result<(), Box<dyn std::error::Error>> {
    let server = support::LiveServer::start().await?;
    let config = OpenAIConfig::new()
        .with_api_base(format!("http://{}/v1", server.addr()))
        .with_api_key("local-test");
    let client = Client::with_config(config);

    let response = client.models().retrieve(support::MODEL_ID).await?;

    assert_eq!(response.id, support::MODEL_ID);
    assert_eq!(response.object, "model");
    assert_eq!(response.owned_by, "ferrite");
    Ok(())
}

#[tokio::test]
async fn async_openai_client_creates_legacy_completion() -> Result<(), Box<dyn std::error::Error>> {
    let server = support::LiveServer::start().await?;
    let config = OpenAIConfig::new()
        .with_api_base(format!("http://{}/v1", server.addr()))
        .with_api_key("local-test");
    let client = Client::with_config(config);

    let response = client
        .completions()
        .create(CreateCompletionRequest {
            model: support::MODEL_ID.to_owned(),
            prompt: Prompt::String("hello".to_owned()),
            max_tokens: Some(1),
            ..Default::default()
        })
        .await?;

    assert_eq!(response.object, "text_completion");
    assert_eq!(response.model, support::MODEL_ID);
    assert_eq!(response.choices[0].text, "winner");
    assert_eq!(
        response.usage.as_ref().map(|usage| usage.completion_tokens),
        Some(1)
    );
    Ok(())
}

#[tokio::test]
async fn async_openai_client_uses_ferrite_base_url() -> Result<(), Box<dyn std::error::Error>> {
    let server = support::LiveServer::start().await?;
    let config = OpenAIConfig::new()
        .with_api_base(format!("http://{}/v1", server.addr()))
        .with_api_key("local-test");
    let client = Client::with_config(config);

    let response = client
        .chat()
        .create(CreateChatCompletionRequest {
            model: support::MODEL_ID.to_owned(),
            messages: vec![ChatCompletionRequestMessage::User(
                ChatCompletionRequestUserMessage {
                    content: "hello".into(),
                    ..Default::default()
                },
            )],
            max_completion_tokens: Some(1),
            ..Default::default()
        })
        .await?;

    assert_eq!(response.object, "chat.completion");
    assert_eq!(response.model, support::MODEL_ID);
    assert_eq!(
        response.choices[0].message.content.as_deref(),
        Some("winner")
    );
    assert_eq!(
        response.usage.as_ref().map(|usage| usage.completion_tokens),
        Some(1)
    );
    Ok(())
}

#[tokio::test]
async fn async_openai_client_uses_api_key_as_ferrite_bearer_token(
) -> Result<(), Box<dyn std::error::Error>> {
    let server = support::LiveServer::start_with_api_key("local-secret").await?;
    let config = OpenAIConfig::new()
        .with_api_base(format!("http://{}/v1", server.addr()))
        .with_api_key("local-secret");
    let client = Client::with_config(config);

    let response = client
        .chat()
        .create(CreateChatCompletionRequest {
            model: support::MODEL_ID.to_owned(),
            messages: vec![ChatCompletionRequestMessage::User(
                ChatCompletionRequestUserMessage {
                    content: "hello".into(),
                    ..Default::default()
                },
            )],
            max_completion_tokens: Some(1),
            ..Default::default()
        })
        .await?;

    assert_eq!(response.object, "chat.completion");
    assert_eq!(response.model, support::MODEL_ID);
    assert_eq!(
        response.choices[0].message.content.as_deref(),
        Some("winner")
    );
    Ok(())
}

#[tokio::test]
async fn async_openai_client_streams_chat_completion() -> Result<(), Box<dyn std::error::Error>> {
    let server = support::LiveServer::start().await?;
    let config = OpenAIConfig::new()
        .with_api_base(format!("http://{}/v1", server.addr()))
        .with_api_key("local-test");
    let client = Client::with_config(config);

    let mut stream = client
        .chat()
        .create_stream(CreateChatCompletionRequest {
            model: support::MODEL_ID.to_owned(),
            messages: vec![ChatCompletionRequestMessage::User(
                ChatCompletionRequestUserMessage {
                    content: "hello".into(),
                    ..Default::default()
                },
            )],
            max_completion_tokens: Some(1),
            stream_options: Some(ChatCompletionStreamOptions {
                include_usage: Some(true),
                include_obfuscation: None,
            }),
            ..Default::default()
        })
        .await?;

    let mut content = String::new();
    let mut usage_completion_tokens = None;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        for choice in chunk.choices {
            if let Some(delta) = choice.delta.content {
                content.push_str(&delta);
            }
        }
        if let Some(usage) = chunk.usage {
            usage_completion_tokens = Some(usage.completion_tokens);
        }
    }

    assert_eq!(content, "winner");
    assert_eq!(usage_completion_tokens, Some(1));
    Ok(())
}
