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

    pub fn hit_token_limit(&self) -> Option<bool> {
        let finish = self.throughput.streaming_finish.as_ref()?;
        let usage = self.throughput.streaming_usage?;

        Some(finish.reason() == "length" && usage.completion_tokens() == self.token_length as u64)
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
    if let Some(hit_token_limit) = result.hit_token_limit() {
        output.push_str(&format!(
            "\nlong_chat_result_hit_token_limit={hit_token_limit}"
        ));
    }
    if let Some(timing) = throughput.streaming_timing {
        output.push_str(&format!(
            "\nlong_chat_result_streaming_token_events={}\nlong_chat_result_time_to_first_token_ms={}\nlong_chat_result_streaming_total_elapsed_ms={}\nlong_chat_result_streaming_tokens_per_second={:.6}\nlong_chat_result_token_latency_min_ms={}\nlong_chat_result_token_latency_p50_ms={}\nlong_chat_result_token_latency_p95_ms={}\nlong_chat_result_token_latency_max_ms={}",
            timing.token_events(),
            timing.time_to_first_token().as_millis(),
            timing.total_elapsed().as_millis(),
            timing.tokens_per_second(),
            timing.min_token_latency().as_millis(),
            timing.p50_token_latency().as_millis(),
            timing.p95_token_latency().as_millis(),
            timing.max_token_latency().as_millis()
        ));
    }
    if let Some(rss) = throughput.rss {
        output.push_str(&format!(
            "\nlong_chat_result_server_rss_before_bytes={}\nlong_chat_result_server_rss_after_bytes={}\nlong_chat_result_server_rss_idle_bytes={}",
            rss.before_bytes(),
            rss.after_bytes(),
            rss.idle_bytes()
        ));
    }
    output
}
