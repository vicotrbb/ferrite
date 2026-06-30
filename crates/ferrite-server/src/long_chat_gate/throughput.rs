use super::{LongChatGateConfig, LongChatScenario};
use crate::throughput_client::ThroughputClientConfig;
use std::{error::Error, ffi::OsString};

impl LongChatGateConfig {
    pub fn throughput_configs(&self) -> Result<Vec<ThroughputClientConfig>, Box<dyn Error>> {
        self.scenarios()
            .iter()
            .map(|scenario| ThroughputClientConfig::parse(self.throughput_args(scenario)))
            .map(|result| result.map_err(|error| Box::new(error) as Box<dyn Error>))
            .collect()
    }

    pub fn throughput_args(&self, scenario: &LongChatScenario<'_>) -> Vec<OsString> {
        [
            "ferrite-openai-throughput",
            "--addr",
            self.addr(),
            "--endpoint",
            "chat-completions",
            "--model",
            scenario.model(),
            "--prompt",
            self.prompt(),
            "--assistant-context",
            self.assistant_context(),
            "--follow-up",
            self.follow_up(),
            "--requests",
            "1",
            "--concurrency",
            "1",
            "--max-tokens",
            &scenario.token_length().to_string(),
            "--stream",
            "--stream-usage",
            "--api-key",
            self.api_key(),
        ]
        .into_iter()
        .map(OsString::from)
        .collect()
    }
}
