use crate::runtime::PromptEvaluationControl;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

static NEXT_STREAM_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum StreamDisconnectPoint {
    BeforeGeneration,
    PromptEvaluation,
    TokenStreaming,
    FinalChunks,
}

impl StreamDisconnectPoint {
    fn as_str(self) -> &'static str {
        match self {
            Self::BeforeGeneration => "before_generation",
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
    pub disconnect_to_finish_ms: Option<u128>,
}

#[derive(Debug)]
pub(super) struct StreamLifecycle {
    request_id: String,
    started: Instant,
    prompt_tokens_started: usize,
    prompt_cancellation_polls: usize,
    prompt_cancellation_closed_polls: usize,
    generated_chunks: usize,
    generated_token_ids: usize,
    disconnect_point: Option<StreamDisconnectPoint>,
    disconnect_observed_at: Option<Instant>,
}

impl StreamLifecycle {
    pub(super) fn new() -> Self {
        let id = NEXT_STREAM_ID.fetch_add(1, Ordering::Relaxed);
        Self {
            request_id: format!("stream-{id}"),
            started: Instant::now(),
            prompt_tokens_started: 0,
            prompt_cancellation_polls: 0,
            prompt_cancellation_closed_polls: 0,
            generated_chunks: 0,
            generated_token_ids: 0,
            disconnect_point: None,
            disconnect_observed_at: None,
        }
    }

    pub(super) fn record_prompt_token_started(&mut self) {
        self.prompt_tokens_started += 1;
    }

    pub(super) fn record_prompt_cancellation_poll(&mut self) {
        self.prompt_cancellation_polls += 1;
    }

    pub(super) fn record_generated_chunk(&mut self, token_ids: usize) {
        self.generated_chunks += 1;
        self.generated_token_ids += token_ids;
    }

    pub(super) fn observe_stream_state(
        &mut self,
        point: StreamDisconnectPoint,
        closed: bool,
    ) -> PromptEvaluationControl {
        if closed {
            self.prompt_cancellation_closed_polls += 1;
            self.record_disconnect(point);
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
        let disconnect_to_finish_ms = self
            .disconnect_observed_at
            .map(|observed_at| observed_at.elapsed().as_millis());
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
            disconnect_to_finish_ms,
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
        format!(
            "openai_stream_lifecycle request_id={} finish_reason={} disconnect_point={} prompt_tokens_started={} prompt_cancellation_polls={} prompt_cancellation_closed_polls={} generated_chunks={} generated_token_ids={} elapsed_ms={} disconnect_to_finish_ms={}",
            self.request_id,
            self.finish_reason.as_str(),
            disconnect_point,
            self.prompt_tokens_started,
            self.prompt_cancellation_polls,
            self.prompt_cancellation_closed_polls,
            self.generated_chunks,
            self.generated_token_ids,
            self.elapsed_ms,
            disconnect_to_finish_ms
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_summary_records_prompt_generation_and_disconnect_state() {
        let mut lifecycle = StreamLifecycle::new();

        lifecycle.record_prompt_token_started();
        lifecycle.record_prompt_cancellation_poll();
        lifecycle.record_prompt_cancellation_poll();
        assert_eq!(
            lifecycle.observe_stream_state(StreamDisconnectPoint::PromptEvaluation, true),
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
        assert!(summary.disconnect_to_finish_ms.is_some());
        assert_eq!(summary.generated_chunks, 1);
        assert_eq!(summary.generated_token_ids, 3);
        assert!(summary.log_line().contains("openai_stream_lifecycle"));
        assert!(summary
            .log_line()
            .contains("disconnect_point=prompt_evaluation"));
        assert!(summary
            .log_line()
            .contains("prompt_cancellation_closed_polls=1"));
        assert!(summary.log_line().contains("disconnect_to_finish_ms="));
    }
}
