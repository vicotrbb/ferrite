use super::{LongChatScenario, LongChatTextIdentity};
use crate::throughput_client::ThroughputResult;

#[derive(Clone, Debug)]
pub struct LongChatScenarioResult {
    model: String,
    turn: usize,
    token_length: usize,
    prompt_cache_key: Option<String>,
    assistant_context_source: LongChatAssistantContextSource,
    assistant_context_identity: Option<LongChatTextIdentity>,
    throughput: ThroughputResult,
}

impl LongChatScenarioResult {
    pub fn new(scenario: &LongChatScenario<'_>, throughput: ThroughputResult) -> Self {
        Self::new_with_assistant_context_source(
            scenario,
            throughput,
            LongChatAssistantContextSource::Seed,
        )
    }

    pub fn new_with_assistant_context_source(
        scenario: &LongChatScenario<'_>,
        throughput: ThroughputResult,
        assistant_context_source: LongChatAssistantContextSource,
    ) -> Self {
        Self::new_with_optional_assistant_context_identity(
            scenario,
            throughput,
            assistant_context_source,
            None,
        )
    }

    pub fn new_with_assistant_context_source_and_identity(
        scenario: &LongChatScenario<'_>,
        throughput: ThroughputResult,
        assistant_context_source: LongChatAssistantContextSource,
        assistant_context_identity: LongChatTextIdentity,
    ) -> Self {
        Self::new_with_optional_assistant_context_identity(
            scenario,
            throughput,
            assistant_context_source,
            Some(assistant_context_identity),
        )
    }

    fn new_with_optional_assistant_context_identity(
        scenario: &LongChatScenario<'_>,
        throughput: ThroughputResult,
        assistant_context_source: LongChatAssistantContextSource,
        assistant_context_identity: Option<LongChatTextIdentity>,
    ) -> Self {
        Self {
            model: scenario.model().to_owned(),
            turn: scenario.turn(),
            token_length: scenario.token_length(),
            prompt_cache_key: scenario.prompt_cache_key().map(str::to_owned),
            assistant_context_source,
            assistant_context_identity,
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

    pub fn prompt_cache_key(&self) -> Option<&str> {
        self.prompt_cache_key.as_deref()
    }

    pub fn assistant_context_source(&self) -> LongChatAssistantContextSource {
        self.assistant_context_source
    }

    pub fn assistant_context_identity(&self) -> Option<LongChatTextIdentity> {
        self.assistant_context_identity
    }

    pub fn throughput(&self) -> &ThroughputResult {
        &self.throughput
    }

    pub fn hit_token_limit(&self) -> Option<bool> {
        let finish = self.throughput.streaming_finish.as_ref()?;
        let usage = self.throughput.streaming_usage.as_ref()?;

        Some(finish.reason() == "length" && usage.completion_tokens() == self.token_length as u64)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LongChatAssistantContextSource {
    Seed,
    Generated,
}

impl LongChatAssistantContextSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Seed => "seed",
            Self::Generated => "generated",
        }
    }

    pub fn is_generated(self) -> bool {
        self == Self::Generated
    }
}

pub fn format_scenario_result(result: &LongChatScenarioResult) -> String {
    let throughput = result.throughput();
    let prompt_cache_key = result
        .prompt_cache_key()
        .map(|key| format!(",prompt_cache_key:{key}"))
        .unwrap_or_default();
    let mut output = format!(
        "long_chat_result=model:{},turn:{},max_tokens:{}{}\nlong_chat_result_assistant_context_source={}\nlong_chat_result_completed_requests={}\nlong_chat_result_elapsed_ms={}",
        result.model(),
        result.turn(),
        result.token_length(),
        prompt_cache_key,
        result.assistant_context_source().as_str(),
        throughput.completed_requests,
        throughput.elapsed.as_millis()
    );
    if let Some(finish) = &throughput.streaming_finish {
        output.push_str(&format!(
            "\nlong_chat_result_finish_reason={}",
            finish.reason()
        ));
    }
    if let Some(identity) = result.assistant_context_identity() {
        output.push_str(&format!(
            "\nlong_chat_result_assistant_context_bytes={}\nlong_chat_result_assistant_context_hash={}",
            identity.byte_len(),
            identity.formatted_hash(),
        ));
    }
    if let Some(text) = &throughput.streaming_text {
        output.push_str(&format!(
            "\nlong_chat_result_generated_response_bytes={}\nlong_chat_result_generated_response_chunks={}\nlong_chat_result_generated_response_hash={}",
            text.byte_len(),
            text.chunk_count(),
            text.formatted_text_hash(),
        ));
    }
    if let Some(usage) = &throughput.streaming_usage {
        output.push_str(&format!(
            "\nlong_chat_result_usage_prompt_tokens={}\nlong_chat_result_usage_cached_prompt_tokens={}\nlong_chat_result_usage_completion_tokens={}\nlong_chat_result_usage_total_tokens={}",
            usage.prompt_tokens(),
            usage.cached_prompt_tokens(),
            usage.completion_tokens(),
            usage.total_tokens()
        ));
        if let Some(trace) = usage.prompt_cache_trace() {
            output.push_str(&format!(
                "\nlong_chat_result_prompt_cache_lookup={}\nlong_chat_result_prompt_cache_prompt_token_hash={}\nlong_chat_result_prompt_cache_shared_prefix_tokens={}",
                trace.lookup(),
                trace.prompt_token_hash(),
                trace.shared_prefix_tokens(),
            ));
            if let Some(selected_entry_token_hash) = trace.selected_entry_token_hash() {
                output.push_str(&format!(
                    "\nlong_chat_result_prompt_cache_selected_entry_token_hash={selected_entry_token_hash}"
                ));
            }
        }
    }
    if let Some(hit_token_limit) = result.hit_token_limit() {
        output.push_str(&format!(
            "\nlong_chat_result_hit_token_limit={hit_token_limit}"
        ));
    }
    if let Some(timing) = throughput.streaming_timing {
        output.push_str(&format!(
            "\nlong_chat_result_streaming_token_events={}\nlong_chat_result_time_to_first_token_ms={}\nlong_chat_result_stream_observed_prefill_elapsed_ms={}\nlong_chat_result_first_token_timestamp_ms={}\nlong_chat_result_stream_observed_decode_elapsed_ms={}\nlong_chat_result_stream_observed_decode_tokens_per_second={:.6}\nlong_chat_result_streaming_total_elapsed_ms={}\nlong_chat_result_streaming_tokens_per_second={:.6}\nlong_chat_result_token_latency_min_ms={}\nlong_chat_result_token_latency_p50_ms={}\nlong_chat_result_token_latency_p95_ms={}\nlong_chat_result_token_latency_max_ms={}",
            timing.token_events(),
            timing.time_to_first_token().as_millis(),
            timing.stream_observed_prefill_elapsed().as_millis(),
            timing.first_token_timestamp().as_millis(),
            timing.stream_observed_decode_elapsed().as_millis(),
            timing.stream_observed_decode_tokens_per_second(),
            timing.total_elapsed().as_millis(),
            timing.tokens_per_second(),
            timing.min_token_latency().as_millis(),
            timing.p50_token_latency().as_millis(),
            timing.p95_token_latency().as_millis(),
            timing.max_token_latency().as_millis()
        ));
    }
    if let Some(token_ids) = throughput.streaming_token_ids {
        output.push_str(&format!(
            "\nlong_chat_result_streaming_content_chunks={}\nlong_chat_result_streaming_token_id_chunks={}\nlong_chat_result_streaming_token_ids={}\nlong_chat_result_streaming_all_content_chunks_have_token_ids={}",
            token_ids.content_chunks(),
            token_ids.token_id_chunks(),
            token_ids.token_ids(),
            token_ids.all_content_chunks_have_token_ids()
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
