mod support;

use async_openai::{config::OpenAIConfig, Client};

fn ferrite_client(server: &support::LiveServer) -> Client<OpenAIConfig> {
    Client::with_config(
        OpenAIConfig::new()
            .with_api_base(format!("http://{}/v1", server.addr()))
            .with_api_key("local-test"),
    )
}

#[tokio::test]
async fn async_openai_client_lists_ferrite_model() -> Result<(), Box<dyn std::error::Error>> {
    let server = support::LiveServer::start().await?;
    let client = ferrite_client(&server);

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
    let client = ferrite_client(&server);

    let response = client.models().retrieve(support::MODEL_ID).await?;

    assert_eq!(response.id, support::MODEL_ID);
    assert_eq!(response.object, "model");
    assert_eq!(response.owned_by, "ferrite");
    Ok(())
}
