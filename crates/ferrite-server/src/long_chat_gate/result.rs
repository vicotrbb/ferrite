use super::LongChatScenario;
use crate::throughput_client::ThroughputResult;

#[derive(Clone, Debug)]
pub struct LongChatScenarioResult {
    model: String,
    turn: usize,
    token_length: usize,
    throughput: ThroughputResult,
}

impl LongChatScenarioResult {
    pub fn new(scenario: &LongChatScenario<'_>, throughput: ThroughputResult) -> Self {
        Self {
            model: scenario.model().to_owned(),
            turn: scenario.turn(),
            token_length: scenario.token_length(),
            throughput,
        }
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn turn(&self) -> usize {
        self.turn
    }

    pub fn token_length(&self) -> usize {
        self.token_length
    }

    pub fn throughput(&self) -> &ThroughputResult {
        &self.throughput
    }
}

pub fn format_scenario_result(result: &LongChatScenarioResult) -> String {
    let throughput = result.throughput();
    let mut output = format!(
        "long_chat_result=model:{},turn:{},max_tokens:{}\nlong_chat_result_completed_requests={}\nlong_chat_result_elapsed_ms={}",
        result.model(),
        result.turn(),
        result.token_length(),
        throughput.completed_requests,
        throughput.elapsed.as_millis()
    );
    if let Some(finish) = &throughput.streaming_finish {
        output.push_str(&format!(
            "\nlong_chat_result_finish_reason={}",
            finish.reason()
        ));
    }
    if let Some(usage) = throughput.streaming_usage {
        output.push_str(&format!(
            "\nlong_chat_result_usage_prompt_tokens={}\nlong_chat_result_usage_completion_tokens={}\nlong_chat_result_usage_total_tokens={}",
            usage.prompt_tokens(),
            usage.completion_tokens(),
            usage.total_tokens()
        ));
    }
    output
}
