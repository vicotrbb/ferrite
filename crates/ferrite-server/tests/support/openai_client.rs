use super::LiveServer;
use async_openai::types::chat::Prompt;
use async_openai::types::{
    chat::{
        ChatCompletionRequestMessage, ChatCompletionRequestUserMessage,
        ChatCompletionStreamOptions, CreateChatCompletionRequest,
    },
    completions::CreateCompletionRequest,
};
use async_openai::{Client, config::OpenAIConfig};
use tokio_stream::StreamExt;

pub fn ferrite_client(server: &LiveServer, api_key: &str) -> Client<OpenAIConfig> {
    Client::with_config(
        OpenAIConfig::new()
            .with_api_base(format!("http://{}/v1", server.addr()))
            .with_api_key(api_key),
    )
}

pub async fn assert_completion_create(
    server: &LiveServer,
    model_id: &str,
    expected_text: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = ferrite_client(server, "local-test");
    let response = client
        .completions()
        .create(completion_request(model_id))
        .await?;

    assert_eq!(response.object, "text_completion");
    assert_eq!(response.model, model_id);
    assert_eq!(response.choices[0].text, expected_text);
    assert_eq!(
        response.usage.as_ref().map(|usage| usage.completion_tokens),
        Some(1)
    );
    Ok(())
}

pub async fn assert_completion_stream(
    server: &LiveServer,
    model_id: &str,
    expected_text: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = ferrite_client(server, "local-test");
    let mut stream = client
        .completions()
        .create_stream(CreateCompletionRequest {
            stream_options: Some(ChatCompletionStreamOptions {
                include_usage: Some(true),
                include_obfuscation: None,
            }),
            ..completion_request(model_id)
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

    assert_eq!(text, expected_text);
    assert_eq!(completion_tokens, Some(1));
    Ok(())
}

pub async fn assert_chat_create(
    server: &LiveServer,
    model_id: &str,
    expected_content: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = ferrite_client(server, "local-test");
    let response = client.chat().create(chat_request(model_id)).await?;

    assert_eq!(response.object, "chat.completion");
    assert_eq!(response.model, model_id);
    assert_eq!(
        response.choices[0].message.content.as_deref(),
        Some(expected_content)
    );
    assert_eq!(
        response.usage.as_ref().map(|usage| usage.completion_tokens),
        Some(1)
    );
    Ok(())
}

pub async fn assert_chat_stream(
    server: &LiveServer,
    model_id: &str,
    expected_content: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = ferrite_client(server, "local-test");
    let mut request = chat_request(model_id);
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

    assert_eq!(content, expected_content);
    assert_eq!(completion_tokens, Some(1));
    Ok(())
}

fn completion_request(model_id: &str) -> CreateCompletionRequest {
    CreateCompletionRequest {
        model: model_id.to_owned(),
        prompt: Prompt::String("hello world".to_owned()),
        max_tokens: Some(1),
        ..Default::default()
    }
}

fn chat_request(model_id: &str) -> CreateChatCompletionRequest {
    CreateChatCompletionRequest {
        model: model_id.to_owned(),
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
