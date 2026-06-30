use super::LiveServer;
use async_openai::{config::OpenAIConfig, Client};

pub fn ferrite_client(server: &LiveServer, api_key: &str) -> Client<OpenAIConfig> {
    Client::with_config(
        OpenAIConfig::new()
            .with_api_base(format!("http://{}/v1", server.addr()))
            .with_api_key(api_key),
    )
}
