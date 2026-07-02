use super::{LongChatAssistantContextSource, LongChatGateConfig, LongChatScenarioResult};
use crate::throughput_client::{
    run_completion_benchmark, ThroughputClientConfig, ThroughputResult,
};
use std::{collections::HashMap, error::Error};

impl LongChatGateConfig {
    pub async fn run(&self) -> Result<Vec<LongChatScenarioResult>, Box<dyn Error>> {
        self.run_with_observer(|_| Ok(())).await
    }

    pub async fn run_with_observer(
        &self,
        mut observer: impl FnMut(&LongChatScenarioResult) -> Result<(), Box<dyn Error>>,
    ) -> Result<Vec<LongChatScenarioResult>, Box<dyn Error>> {
        let scenarios = self.scenarios();
        let mut assistant_contexts = LongChatAssistantContexts::new(
            self.assistant_context(),
            self.generated_context_max_chars(),
            self.generated_context_max_tokens(),
        );
        let mut results = Vec::with_capacity(scenarios.len());

        for scenario in &scenarios {
            let assistant_context = assistant_contexts.context_for(scenario);
            let throughput_config = self.throughput_config_with_assistant_context(
                scenario,
                assistant_context.text.as_str(),
            )?;
            let throughput = run_completion_benchmark(&throughput_config).await?;
            self.validate_finish_reason(&throughput)?;
            assistant_contexts.record_result(scenario, &throughput);
            let result = LongChatScenarioResult::new_with_assistant_context_source(
                scenario,
                throughput,
                assistant_context.source,
            );
            observer(&result)?;
            results.push(result);
        }

        Ok(results)
    }

    pub fn run_with_executor(
        &self,
        mut executor: impl FnMut(&ThroughputClientConfig) -> Result<ThroughputResult, Box<dyn Error>>,
    ) -> Result<Vec<LongChatScenarioResult>, Box<dyn Error>> {
        self.run_with_executor_and_observer(&mut executor, |_| Ok(()))
    }

    pub fn run_with_executor_and_observer(
        &self,
        mut executor: impl FnMut(&ThroughputClientConfig) -> Result<ThroughputResult, Box<dyn Error>>,
        mut observer: impl FnMut(&LongChatScenarioResult) -> Result<(), Box<dyn Error>>,
    ) -> Result<Vec<LongChatScenarioResult>, Box<dyn Error>> {
        let scenarios = self.scenarios();
        let mut assistant_contexts = LongChatAssistantContexts::new(
            self.assistant_context(),
            self.generated_context_max_chars(),
            self.generated_context_max_tokens(),
        );
        let mut results = Vec::with_capacity(scenarios.len());

        for scenario in &scenarios {
            let assistant_context = assistant_contexts.context_for(scenario);
            let throughput_config = self.throughput_config_with_assistant_context(
                scenario,
                assistant_context.text.as_str(),
            )?;
            let throughput = executor(&throughput_config)?;
            self.validate_finish_reason(&throughput)?;
            assistant_contexts.record_result(scenario, &throughput);
            let result = LongChatScenarioResult::new_with_assistant_context_source(
                scenario,
                throughput,
                assistant_context.source,
            );
            observer(&result)?;
            results.push(result);
        }

        Ok(results)
    }

    fn validate_finish_reason(&self, throughput: &ThroughputResult) -> Result<(), Box<dyn Error>> {
        let Some(expected) = self.expected_finish_reason() else {
            return Ok(());
        };
        let Some(actual) = throughput
            .streaming_finish
            .as_ref()
            .map(|finish| finish.reason())
        else {
            return Err(format!("expected finish_reason {expected}, got none").into());
        };
        if actual != expected {
            return Err(format!("expected finish_reason {expected}, got {actual}").into());
        }
        Ok(())
    }
}

struct LongChatAssistantContexts {
    seed: String,
    generated_context_max_chars: Option<usize>,
    generated_context_max_tokens: Option<usize>,
    generated_by_scenario: HashMap<(String, usize), String>,
}

struct LongChatAssistantContext {
    text: String,
    source: LongChatAssistantContextSource,
}

impl LongChatAssistantContexts {
    fn new(
        seed: &str,
        generated_context_max_chars: Option<usize>,
        generated_context_max_tokens: Option<usize>,
    ) -> Self {
        Self {
            seed: seed.to_owned(),
            generated_context_max_chars,
            generated_context_max_tokens,
            generated_by_scenario: HashMap::new(),
        }
    }

    fn context_for(&self, scenario: &super::LongChatScenario<'_>) -> LongChatAssistantContext {
        if let Some(text) = self
            .generated_by_scenario
            .get(&(scenario.model().to_owned(), scenario.token_length()))
        {
            return LongChatAssistantContext {
                text: text.clone(),
                source: LongChatAssistantContextSource::Generated,
            };
        }

        LongChatAssistantContext {
            text: self.seed.clone(),
            source: LongChatAssistantContextSource::Seed,
        }
    }

    fn record_result(&mut self, scenario: &super::LongChatScenario<'_>, result: &ThroughputResult) {
        let Some(text) = &result.streaming_text else {
            return;
        };
        let text = match (
            self.generated_context_max_tokens,
            self.generated_context_max_chars,
        ) {
            (Some(max_tokens), _) => trailing_chunks(text.chunks(), max_tokens),
            (None, Some(max_chars)) => trailing_chars(text.text(), max_chars),
            (None, None) => text.text().to_owned(),
        };
        self.generated_by_scenario
            .insert((scenario.model().to_owned(), scenario.token_length()), text);
    }
}

fn trailing_chars(text: &str, max_chars: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= max_chars {
        return text.to_owned();
    }
    text.chars().skip(char_count - max_chars).collect()
}

fn trailing_chunks(chunks: &[String], max_chunks: usize) -> String {
    let start = chunks.len().saturating_sub(max_chunks);
    chunks[start..].concat()
}
