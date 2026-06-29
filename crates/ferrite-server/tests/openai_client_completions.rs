mod support;

use async_openai::types::chat::Prompt;
use async_openai::{
    config::OpenAIConfig,
    types::{chat::ChatCompletionStreamOptions, completions::CreateCompletionRequest},
    Client,
};
use tokio_stream::StreamExt;

fn ferrite_client(server: &support::LiveServer) -> Client<OpenAIConfig> {
    Client::with_config(
        OpenAIConfig::new()
            .with_api_base(format!("http://{}/v1", server.addr()))
            .with_api_key("local-test"),
    )
}

#[tokio::test]
async fn async_openai_client_creates_legacy_completion() -> Result<(), Box<dyn std::error::Error>> {
    let server = support::LiveServer::start().await?;
    let client = ferrite_client(&server);

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
async fn async_openai_client_streams_legacy_completion() -> Result<(), Box<dyn std::error::Error>> {
    let server = support::LiveServer::start().await?;
    let client = ferrite_client(&server);

    let mut stream = client
        .completions()
        .create_stream(CreateCompletionRequest {
            model: support::MODEL_ID.to_owned(),
            prompt: Prompt::String("hello".to_owned()),
            max_tokens: Some(1),
            stream_options: Some(ChatCompletionStreamOptions {
                include_usage: Some(true),
                include_obfuscation: None,
            }),
            ..Default::default()
        })
        .await?;

    let mut text = String::new();
    let mut usage_completion_tokens = None;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        for choice in chunk.choices {
            text.push_str(&choice.text);
        }
        if let Some(usage) = chunk.usage {
            usage_completion_tokens = Some(usage.completion_tokens);
        }
    }

    assert_eq!(text, "winner");
    assert_eq!(usage_completion_tokens, Some(1));
    Ok(())
}
