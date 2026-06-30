mod support;

use async_openai::types::chat::{
    ChatCompletionRequestMessage, ChatCompletionRequestUserMessage, ChatCompletionStreamOptions,
    CreateChatCompletionRequest,
};
use support::openai_client::ferrite_client;
use tokio_stream::StreamExt;

fn chat_request() -> CreateChatCompletionRequest {
    CreateChatCompletionRequest {
        model: support::MODEL_ID.to_owned(),
        messages: vec![ChatCompletionRequestMessage::User(
            ChatCompletionRequestUserMessage {
                content: "hello".into(),
                ..Default::default()
            },
        )],
        max_completion_tokens: Some(1),
        ..Default::default()
    }
}

#[tokio::test]
async fn async_openai_client_uses_ferrite_base_url() -> Result<(), Box<dyn std::error::Error>> {
    let server = support::LiveServer::start().await?;
    let client = ferrite_client(&server, "local-test");

    let response = client.chat().create(chat_request()).await?;

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
    let client = ferrite_client(&server, "local-secret");

    let response = client.chat().create(chat_request()).await?;

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
    let client = ferrite_client(&server, "local-test");
    let mut request = chat_request();
    request.stream_options = Some(ChatCompletionStreamOptions {
        include_usage: Some(true),
        include_obfuscation: None,
    });

    let mut stream = client.chat().create_stream(request).await?;

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

#[tokio::test]
async fn async_openai_client_streams_chat_completion_with_obfuscation(
) -> Result<(), Box<dyn std::error::Error>> {
    let server = support::LiveServer::start().await?;
    let client = ferrite_client(&server, "local-test");
    let mut request = chat_request();
    request.stream_options = Some(ChatCompletionStreamOptions {
        include_usage: Some(true),
        include_obfuscation: Some(true),
    });

    let mut stream = client.chat().create_stream(request).await?;

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
