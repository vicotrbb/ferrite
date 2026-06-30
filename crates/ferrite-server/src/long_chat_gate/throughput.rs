use super::{LongChatGateConfig, LongChatScenario};
use std::ffi::OsString;

impl LongChatGateConfig {
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
