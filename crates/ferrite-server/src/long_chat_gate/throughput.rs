use super::{LongChatGateConfig, LongChatScenario};
use crate::throughput_client::ThroughputClientConfig;
use std::{error::Error, ffi::OsString};

impl LongChatGateConfig {
    pub fn throughput_configs(&self) -> Result<Vec<ThroughputClientConfig>, Box<dyn Error>> {
        self.scenarios()
            .iter()
            .map(|scenario| {
                self.throughput_config_with_assistant_context(scenario, self.assistant_context())
            })
            .collect()
    }

    pub fn throughput_config_with_assistant_context(
        &self,
        scenario: &LongChatScenario<'_>,
        assistant_context: &str,
    ) -> Result<ThroughputClientConfig, Box<dyn Error>> {
        self.throughput_config_with_chat_context(scenario, assistant_context, self.follow_up())
    }

    pub fn throughput_config_with_chat_context(
        &self,
        scenario: &LongChatScenario<'_>,
        assistant_context: &str,
        follow_up: &str,
    ) -> Result<ThroughputClientConfig, Box<dyn Error>> {
        ThroughputClientConfig::parse(self.throughput_args_with_chat_context(
            scenario,
            assistant_context,
            follow_up,
        ))
        .map_err(|error| Box::new(error) as Box<dyn Error>)
    }

    pub fn throughput_args(&self, scenario: &LongChatScenario<'_>) -> Vec<OsString> {
        self.throughput_args_with_assistant_context(scenario, self.assistant_context())
    }

    fn throughput_args_with_assistant_context(
        &self,
        scenario: &LongChatScenario<'_>,
        assistant_context: &str,
    ) -> Vec<OsString> {
        self.throughput_args_with_chat_context(scenario, assistant_context, self.follow_up())
    }

    fn throughput_args_with_chat_context(
        &self,
        scenario: &LongChatScenario<'_>,
        assistant_context: &str,
        follow_up: &str,
    ) -> Vec<OsString> {
        let mut args: Vec<OsString> = [
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
            assistant_context,
            "--follow-up",
            follow_up,
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
        .collect();

        if let Some(stop) = self.stop() {
            args.extend([OsString::from("--stop"), OsString::from(stop)]);
        }
        if let Some(prompt_cache_key) = scenario.prompt_cache_key().or(self.prompt_cache_key()) {
            args.extend([
                OsString::from("--prompt-cache-key"),
                OsString::from(prompt_cache_key),
            ]);
        }
        if self.prompt_cache_trace() {
            args.push(OsString::from("--prompt-cache-trace"));
        }
        if let Some(rss_pid) = self.rss_pid() {
            args.extend([
                OsString::from("--rss-pid"),
                OsString::from(rss_pid.to_string()),
            ]);
        }

        args
    }
}
