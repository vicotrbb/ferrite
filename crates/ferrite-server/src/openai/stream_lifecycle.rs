use crate::runtime::{GenerationStage, PromptEvaluationControl};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

static NEXT_STREAM_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum StreamDisconnectPoint {
    BeforeGeneration,
    Tokenization,
    PromptEvaluation,
    TokenStreaming,
    FinalChunks,
}

impl StreamDisconnectPoint {
    fn as_str(self) -> &'static str {
        match self {
            Self::BeforeGeneration => "before_generation",
            Self::Tokenization => "tokenization",
            Self::PromptEvaluation => "prompt_evaluation",
            Self::TokenStreaming => "token_streaming",
            Self::FinalChunks => "final_chunks",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum StreamFinishReason {
    Completed,
    Cancelled,
    Failed,
}

impl StreamFinishReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct StreamLifecycleSummary {
    pub request_id: String,
    pub finish_reason: StreamFinishReason,
    pub disconnect_point: Option<StreamDisconnectPoint>,
    pub prompt_tokens_started: usize,
    pub prompt_cancellation_polls: usize,
    pub prompt_cancellation_closed_polls: usize,
    pub generated_chunks: usize,
    pub generated_token_ids: usize,
    pub elapsed_ms: u128,
    pub disconnect_observed_elapsed_ms: Option<u128>,
    pub disconnect_to_finish_ms: Option<u128>,
    pub prompt_cancellation_token_index: Option<usize>,
    pub prompt_cancellation_layer_index: Option<usize>,
    pub engine_lock_acquired_elapsed_ms: Option<u128>,
    pub generation_started_elapsed_ms: Option<u128>,
    pub prompt_tokenized_elapsed_ms: Option<u128>,
    pub prefix_cache_key_built_elapsed_ms: Option<u128>,
    pub session_started_elapsed_ms: Option<u128>,
    pub prefix_cache_lookup_finished_elapsed_ms: Option<u128>,
    pub prefix_cache_restored_elapsed_ms: Option<u128>,
    pub prompt_evaluation_started_elapsed_ms: Option<u128>,
    pub first_prompt_token_started_elapsed_ms: Option<u128>,
    pub first_prompt_cancellation_poll_elapsed_ms: Option<u128>,
}

#[derive(Debug)]
pub(super) struct StreamLifecycle {
    request_id: String,
    started: Instant,
    engine_lock_acquired_at: Option<Instant>,
    generation_started_at: Option<Instant>,
    prompt_tokenized_at: Option<Instant>,
    prefix_cache_key_built_at: Option<Instant>,
    session_started_at: Option<Instant>,
    prefix_cache_lookup_finished_at: Option<Instant>,
    prefix_cache_restored_at: Option<Instant>,
    prompt_evaluation_started_at: Option<Instant>,
    first_prompt_token_started_at: Option<Instant>,
    first_prompt_cancellation_poll_at: Option<Instant>,
    prompt_tokens_started: usize,
    prompt_cancellation_polls: usize,
    prompt_cancellation_closed_polls: usize,
    generated_chunks: usize,
    generated_token_ids: usize,
    disconnect_point: Option<StreamDisconnectPoint>,
    disconnect_observed_at: Option<Instant>,
    prompt_cancellation_token_index: Option<usize>,
    prompt_cancellation_layer_index: Option<usize>,
}

impl StreamLifecycle {
    pub(super) fn new() -> Self {
        let id = NEXT_STREAM_ID.fetch_add(1, Ordering::Relaxed);
        Self {
            request_id: format!("stream-{id}"),
            started: Instant::now(),
            engine_lock_acquired_at: None,
            generation_started_at: None,
            prompt_tokenized_at: None,
            prefix_cache_key_built_at: None,
            session_started_at: None,
            prefix_cache_lookup_finished_at: None,
            prefix_cache_restored_at: None,
            prompt_evaluation_started_at: None,
            first_prompt_token_started_at: None,
            first_prompt_cancellation_poll_at: None,
            prompt_tokens_started: 0,
            prompt_cancellation_polls: 0,
            prompt_cancellation_closed_polls: 0,
            generated_chunks: 0,
            generated_token_ids: 0,
            disconnect_point: None,
            disconnect_observed_at: None,
            prompt_cancellation_token_index: None,
            prompt_cancellation_layer_index: None,
        }
    }

    pub(super) fn record_engine_lock_acquired(&mut self) {
        self.engine_lock_acquired_at
            .get_or_insert_with(Instant::now);
    }

    pub(super) fn record_generation_started(&mut self) {
        self.generation_started_at.get_or_insert_with(Instant::now);
    }

    pub(super) fn record_generation_stage(&mut self, stage: GenerationStage) {
        match stage {
            GenerationStage::PromptTokenized => {
                self.prompt_tokenized_at.get_or_insert_with(Instant::now);
            }
            GenerationStage::PrefixCacheKeyBuilt => {
                self.prefix_cache_key_built_at
                    .get_or_insert_with(Instant::now);
            }
            GenerationStage::SessionStarted => {
                self.session_started_at.get_or_insert_with(Instant::now);
            }
            GenerationStage::PrefixCacheLookupFinished => {
                self.prefix_cache_lookup_finished_at
                    .get_or_insert_with(Instant::now);
            }
            GenerationStage::PrefixCacheRestored => {
                self.prefix_cache_restored_at
                    .get_or_insert_with(Instant::now);
            }
            GenerationStage::PromptEvaluationStarted => {
                self.prompt_evaluation_started_at
                    .get_or_insert_with(Instant::now);
            }
        }
    }

    pub(super) fn record_prompt_token_started(&mut self) {
        self.first_prompt_token_started_at
            .get_or_insert_with(Instant::now);
        self.prompt_tokens_started += 1;
    }

    pub(super) fn record_prompt_cancellation_poll(&mut self) {
        self.first_prompt_cancellation_poll_at
            .get_or_insert_with(Instant::now);
        self.prompt_cancellation_polls += 1;
    }

    pub(super) fn record_generated_chunk(&mut self, token_ids: usize) {
        self.generated_chunks += 1;
        self.generated_token_ids += token_ids;
    }

    pub(super) fn observe_stream_state(
        &mut self,
        point: StreamDisconnectPoint,
        prompt_token_index: usize,
        layer_index: Option<usize>,
        closed: bool,
    ) -> PromptEvaluationControl {
        if closed {
            self.prompt_cancellation_closed_polls += 1;
            if self.prompt_cancellation_token_index.is_none() {
                self.prompt_cancellation_token_index = Some(prompt_token_index);
                self.prompt_cancellation_layer_index = layer_index;
            }
            self.record_disconnect(point);
            PromptEvaluationControl::Cancel
        } else {
            PromptEvaluationControl::Continue
        }
    }

    pub(super) fn observe_tokenization_stream_state(
        &mut self,
        closed: bool,
    ) -> PromptEvaluationControl {
        if closed {
            self.record_disconnect(StreamDisconnectPoint::Tokenization);
            PromptEvaluationControl::Cancel
        } else {
            PromptEvaluationControl::Continue
        }
    }

    pub(super) fn record_disconnect(&mut self, point: StreamDisconnectPoint) {
        if self.disconnect_point.is_none() {
            self.disconnect_point = Some(point);
            self.disconnect_observed_at = Some(Instant::now());
        }
    }

    pub(super) fn has_disconnect(&self) -> bool {
        self.disconnect_point.is_some()
    }

    pub(super) fn finish(&self, finish_reason: StreamFinishReason) -> StreamLifecycleSummary {
        let elapsed_ms = self.started.elapsed().as_millis();
        let disconnect_observed_elapsed_ms = self
            .disconnect_observed_at
            .map(|observed_at| observed_at.duration_since(self.started).as_millis());
        let disconnect_to_finish_ms = self
            .disconnect_observed_at
            .map(|observed_at| observed_at.elapsed().as_millis());
        let engine_lock_acquired_elapsed_ms = self
            .engine_lock_acquired_at
            .map(|instant| instant.duration_since(self.started).as_millis());
        let generation_started_elapsed_ms = self
            .generation_started_at
            .map(|instant| instant.duration_since(self.started).as_millis());
        let prompt_tokenized_elapsed_ms = self
            .prompt_tokenized_at
            .map(|instant| instant.duration_since(self.started).as_millis());
        let prefix_cache_key_built_elapsed_ms = self
            .prefix_cache_key_built_at
            .map(|instant| instant.duration_since(self.started).as_millis());
        let session_started_elapsed_ms = self
            .session_started_at
            .map(|instant| instant.duration_since(self.started).as_millis());
        let prefix_cache_lookup_finished_elapsed_ms = self
            .prefix_cache_lookup_finished_at
            .map(|instant| instant.duration_since(self.started).as_millis());
        let prefix_cache_restored_elapsed_ms = self
            .prefix_cache_restored_at
            .map(|instant| instant.duration_since(self.started).as_millis());
        let prompt_evaluation_started_elapsed_ms = self
            .prompt_evaluation_started_at
            .map(|instant| instant.duration_since(self.started).as_millis());
        let first_prompt_token_started_elapsed_ms = self
            .first_prompt_token_started_at
            .map(|instant| instant.duration_since(self.started).as_millis());
        let first_prompt_cancellation_poll_elapsed_ms = self
            .first_prompt_cancellation_poll_at
            .map(|instant| instant.duration_since(self.started).as_millis());
        StreamLifecycleSummary {
            request_id: self.request_id.clone(),
            finish_reason,
            disconnect_point: self.disconnect_point,
            prompt_tokens_started: self.prompt_tokens_started,
            prompt_cancellation_polls: self.prompt_cancellation_polls,
            prompt_cancellation_closed_polls: self.prompt_cancellation_closed_polls,
            generated_chunks: self.generated_chunks,
            generated_token_ids: self.generated_token_ids,
            elapsed_ms,
            disconnect_observed_elapsed_ms,
            disconnect_to_finish_ms,
            prompt_cancellation_token_index: self.prompt_cancellation_token_index,
            prompt_cancellation_layer_index: self.prompt_cancellation_layer_index,
            engine_lock_acquired_elapsed_ms,
            generation_started_elapsed_ms,
            prompt_tokenized_elapsed_ms,
            prefix_cache_key_built_elapsed_ms,
            session_started_elapsed_ms,
            prefix_cache_lookup_finished_elapsed_ms,
            prefix_cache_restored_elapsed_ms,
            prompt_evaluation_started_elapsed_ms,
            first_prompt_token_started_elapsed_ms,
            first_prompt_cancellation_poll_elapsed_ms,
        }
    }
}

impl StreamLifecycleSummary {
    pub(super) fn log_line(&self) -> String {
        let disconnect_point = self
            .disconnect_point
            .map(StreamDisconnectPoint::as_str)
            .unwrap_or("none");
        let disconnect_to_finish_ms = self
            .disconnect_to_finish_ms
            .map(|elapsed_ms| elapsed_ms.to_string())
            .unwrap_or_else(|| "none".to_string());
        let disconnect_observed_elapsed_ms = self
            .disconnect_observed_elapsed_ms
            .map(|elapsed_ms| elapsed_ms.to_string())
            .unwrap_or_else(|| "none".to_string());
        let prompt_cancellation_token_index = self
            .prompt_cancellation_token_index
            .map(|index| index.to_string())
            .unwrap_or_else(|| "none".to_string());
        let prompt_cancellation_layer_index = self
            .prompt_cancellation_layer_index
            .map(|index| index.to_string())
            .unwrap_or_else(|| "none".to_string());
        let engine_lock_acquired_elapsed_ms =
            format_optional_ms(self.engine_lock_acquired_elapsed_ms);
        let generation_started_elapsed_ms = format_optional_ms(self.generation_started_elapsed_ms);
        let prompt_tokenized_elapsed_ms = format_optional_ms(self.prompt_tokenized_elapsed_ms);
        let prefix_cache_key_built_elapsed_ms =
            format_optional_ms(self.prefix_cache_key_built_elapsed_ms);
        let session_started_elapsed_ms = format_optional_ms(self.session_started_elapsed_ms);
        let prefix_cache_lookup_finished_elapsed_ms =
            format_optional_ms(self.prefix_cache_lookup_finished_elapsed_ms);
        let prefix_cache_restored_elapsed_ms =
            format_optional_ms(self.prefix_cache_restored_elapsed_ms);
        let prompt_evaluation_started_elapsed_ms =
            format_optional_ms(self.prompt_evaluation_started_elapsed_ms);
        let first_prompt_token_started_elapsed_ms =
            format_optional_ms(self.first_prompt_token_started_elapsed_ms);
        let first_prompt_cancellation_poll_elapsed_ms =
            format_optional_ms(self.first_prompt_cancellation_poll_elapsed_ms);
        format!(
            "openai_stream_lifecycle request_id={} finish_reason={} disconnect_point={} prompt_tokens_started={} prompt_cancellation_polls={} prompt_cancellation_closed_polls={} generated_chunks={} generated_token_ids={} elapsed_ms={} disconnect_observed_elapsed_ms={} disconnect_to_finish_ms={} prompt_cancellation_token_index={} prompt_cancellation_layer_index={} engine_lock_acquired_elapsed_ms={} generation_started_elapsed_ms={} prompt_tokenized_elapsed_ms={} prefix_cache_key_built_elapsed_ms={} session_started_elapsed_ms={} prefix_cache_lookup_finished_elapsed_ms={} prefix_cache_restored_elapsed_ms={} prompt_evaluation_started_elapsed_ms={} first_prompt_token_started_elapsed_ms={} first_prompt_cancellation_poll_elapsed_ms={}",
            self.request_id,
            self.finish_reason.as_str(),
            disconnect_point,
            self.prompt_tokens_started,
            self.prompt_cancellation_polls,
            self.prompt_cancellation_closed_polls,
            self.generated_chunks,
            self.generated_token_ids,
            self.elapsed_ms,
            disconnect_observed_elapsed_ms,
            disconnect_to_finish_ms,
            prompt_cancellation_token_index,
            prompt_cancellation_layer_index,
            engine_lock_acquired_elapsed_ms,
            generation_started_elapsed_ms,
            prompt_tokenized_elapsed_ms,
            prefix_cache_key_built_elapsed_ms,
            session_started_elapsed_ms,
            prefix_cache_lookup_finished_elapsed_ms,
            prefix_cache_restored_elapsed_ms,
            prompt_evaluation_started_elapsed_ms,
            first_prompt_token_started_elapsed_ms,
            first_prompt_cancellation_poll_elapsed_ms
        )
    }
}

fn format_optional_ms(value: Option<u128>) -> String {
    value
        .map(|elapsed_ms| elapsed_ms.to_string())
        .unwrap_or_else(|| "none".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_summary_records_prompt_generation_and_disconnect_state() {
        let mut lifecycle = StreamLifecycle::new();

        lifecycle.record_engine_lock_acquired();
        lifecycle.record_generation_started();
        lifecycle.record_generation_stage(GenerationStage::PromptTokenized);
        lifecycle.record_generation_stage(GenerationStage::PrefixCacheKeyBuilt);
        lifecycle.record_generation_stage(GenerationStage::SessionStarted);
        lifecycle.record_generation_stage(GenerationStage::PromptEvaluationStarted);
        lifecycle.record_prompt_token_started();
        lifecycle.record_prompt_cancellation_poll();
        lifecycle.record_prompt_cancellation_poll();
        assert_eq!(
            lifecycle.observe_stream_state(
                StreamDisconnectPoint::PromptEvaluation,
                4,
                Some(7),
                true
            ),
            PromptEvaluationControl::Cancel
        );
        lifecycle.record_generated_chunk(3);

        let summary = lifecycle.finish(StreamFinishReason::Cancelled);

        assert!(summary.request_id.starts_with("stream-"));
        assert_eq!(summary.finish_reason, StreamFinishReason::Cancelled);
        assert_eq!(
            summary.disconnect_point,
            Some(StreamDisconnectPoint::PromptEvaluation)
        );
        assert_eq!(summary.prompt_tokens_started, 1);
        assert_eq!(summary.prompt_cancellation_polls, 2);
        assert_eq!(summary.prompt_cancellation_closed_polls, 1);
        assert!(summary.engine_lock_acquired_elapsed_ms.is_some());
        assert!(summary.generation_started_elapsed_ms.is_some());
        assert!(summary.prompt_tokenized_elapsed_ms.is_some());
        assert!(summary.prefix_cache_key_built_elapsed_ms.is_some());
        assert!(summary.session_started_elapsed_ms.is_some());
        assert!(summary.prompt_evaluation_started_elapsed_ms.is_some());
        assert!(summary.first_prompt_token_started_elapsed_ms.is_some());
        assert!(summary.first_prompt_cancellation_poll_elapsed_ms.is_some());
        assert!(summary.disconnect_observed_elapsed_ms.is_some());
        assert!(summary.disconnect_to_finish_ms.is_some());
        assert_eq!(summary.prompt_cancellation_token_index, Some(4));
        assert_eq!(summary.prompt_cancellation_layer_index, Some(7));
        assert_eq!(summary.generated_chunks, 1);
        assert_eq!(summary.generated_token_ids, 3);
        assert!(summary.log_line().contains("openai_stream_lifecycle"));
        assert!(summary
            .log_line()
            .contains("disconnect_point=prompt_evaluation"));
        assert!(summary
            .log_line()
            .contains("prompt_cancellation_closed_polls=1"));
        assert!(summary
            .log_line()
            .contains("disconnect_observed_elapsed_ms="));
        assert!(summary.log_line().contains("disconnect_to_finish_ms="));
        assert!(summary
            .log_line()
            .contains("prompt_cancellation_token_index=4"));
        assert!(summary
            .log_line()
            .contains("prompt_cancellation_layer_index=7"));
        assert!(summary
            .log_line()
            .contains("engine_lock_acquired_elapsed_ms="));
        assert!(summary
            .log_line()
            .contains("generation_started_elapsed_ms="));
        assert!(summary
            .log_line()
            .contains("first_prompt_token_started_elapsed_ms="));
        assert!(summary
            .log_line()
            .contains("first_prompt_cancellation_poll_elapsed_ms="));
        assert!(summary.log_line().contains("prompt_tokenized_elapsed_ms="));
        assert!(summary
            .log_line()
            .contains("prefix_cache_key_built_elapsed_ms="));
        assert!(summary.log_line().contains("session_started_elapsed_ms="));
        assert!(summary
            .log_line()
            .contains("prompt_evaluation_started_elapsed_ms="));
    }

    #[test]
    fn lifecycle_summary_records_tokenization_disconnect_state() {
        let mut lifecycle = StreamLifecycle::new();

        assert_eq!(
            lifecycle.observe_tokenization_stream_state(true),
            PromptEvaluationControl::Cancel
        );
        let summary = lifecycle.finish(StreamFinishReason::Cancelled);

        assert_eq!(
            summary.disconnect_point,
            Some(StreamDisconnectPoint::Tokenization)
        );
        assert!(summary.log_line().contains("disconnect_point=tokenization"));
    }
}
