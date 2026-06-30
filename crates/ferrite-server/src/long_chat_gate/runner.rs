use super::{LongChatGateConfig, LongChatScenarioResult};
use crate::throughput_client::{
    run_completion_benchmark, ThroughputClientConfig, ThroughputResult,
};
use std::error::Error;

impl LongChatGateConfig {
    pub async fn run(&self) -> Result<Vec<LongChatScenarioResult>, Box<dyn Error>> {
        let scenarios = self.scenarios();
        let throughput_configs = self.throughput_configs()?;
        let mut results = Vec::with_capacity(scenarios.len());

        for (scenario, throughput_config) in scenarios.iter().zip(throughput_configs.iter()) {
            let throughput = run_completion_benchmark(throughput_config).await?;
            self.validate_finish_reason(&throughput)?;
            results.push(LongChatScenarioResult::new(scenario, throughput));
        }

        Ok(results)
    }

    pub fn run_with_executor(
        &self,
        mut executor: impl FnMut(&ThroughputClientConfig) -> Result<ThroughputResult, Box<dyn Error>>,
    ) -> Result<Vec<LongChatScenarioResult>, Box<dyn Error>> {
        let scenarios = self.scenarios();
        let throughput_configs = self.throughput_configs()?;
        let mut results = Vec::with_capacity(scenarios.len());

        for (scenario, throughput_config) in scenarios.iter().zip(throughput_configs.iter()) {
            let throughput = executor(throughput_config)?;
            self.validate_finish_reason(&throughput)?;
            results.push(LongChatScenarioResult::new(scenario, throughput));
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
