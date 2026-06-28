mod support;

use async_openai::{
    config::OpenAIConfig,
    types::chat::{
        ChatCompletionRequestMessage, ChatCompletionRequestUserMessage, CreateChatCompletionRequest,
    },
    Client,
};

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
