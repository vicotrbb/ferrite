use crate::limits::TokenLimits;
use crate::runtime::{BatchScheduler, InferenceEngine, RuntimeError};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

const INFERENCE_PERMITS: usize = 1;

#[derive(Clone, Debug)]
pub struct ServerState {
    model_id: String,
    engine: Option<Arc<InferenceEngine>>,
    inference_permits: Arc<Semaphore>,
    batch_scheduler: Option<Arc<BatchScheduler>>,
    batch_admission_permits: Option<Arc<Semaphore>>,
    inference_wait_timeout: Duration,
    api_key: Option<Arc<str>>,
    token_limits: TokenLimits,
    prefix_cache_enabled: bool,
}

impl ServerState {
    pub fn new(model_id: String) -> Self {
        Self {
            model_id,
            engine: None,
            inference_permits: Arc::new(Semaphore::new(INFERENCE_PERMITS)),
            batch_scheduler: None,
            batch_admission_permits: None,
            inference_wait_timeout: Duration::ZERO,
            api_key: None,
            token_limits: TokenLimits::default(),
            prefix_cache_enabled: false,
        }
    }

    pub fn with_engine(model_id: String, engine: InferenceEngine) -> Self {
        Self {
            model_id,
            engine: Some(Arc::new(engine)),
            inference_permits: Arc::new(Semaphore::new(INFERENCE_PERMITS)),
            batch_scheduler: None,
            batch_admission_permits: None,
            inference_wait_timeout: Duration::ZERO,
            api_key: None,
            token_limits: TokenLimits::default(),
            prefix_cache_enabled: false,
        }
    }

    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(Arc::from(api_key.into()));
        self
    }

    pub fn with_token_limits(mut self, token_limits: TokenLimits) -> Self {
        self.token_limits = token_limits;
        self
    }

    pub fn with_inference_wait_timeout(mut self, timeout: Duration) -> Self {
        self.inference_wait_timeout = timeout;
        self
    }

    pub fn with_prefix_cache_enabled(mut self, enabled: bool) -> Self {
        self.prefix_cache_enabled = enabled;
        self
    }

    /// Sets how many generations may run concurrently. Counts of zero are
    /// clamped to one so the server can always make progress.
    pub fn with_max_concurrent_inferences(mut self, permits: usize) -> Self {
        self.inference_permits = Arc::new(Semaphore::new(permits.max(1)));
        self
    }

    /// Enables scheduler-owned continuous batching for eligible fused-greedy
    /// requests. This is deliberately separate from the default inference
    /// semaphore used by sampling policies that require complete logits.
    pub fn with_batched_decode(self, max_batch_streams: usize) -> Result<Self, RuntimeError> {
        self.with_batched_decode_and_queue(max_batch_streams, max_batch_streams)
    }

    pub fn with_batched_decode_and_queue(
        mut self,
        max_batch_streams: usize,
        max_queued_jobs: usize,
    ) -> Result<Self, RuntimeError> {
        let engine = self
            .engine
            .clone()
            .ok_or_else(|| RuntimeError::new("batched decode requires a loaded model"))?;
        if !engine.batch_decode_compatible() {
            return Err(RuntimeError::new(
                "batched decode requires the default activation matvec policy",
            ));
        }
        let max_batch_streams = max_batch_streams.max(1);
        let max_queued_jobs = max_queued_jobs.max(1);
        self.batch_scheduler = Some(Arc::new(BatchScheduler::start_with_queue(
            engine,
            max_batch_streams,
            max_queued_jobs,
        )?));
        self.batch_admission_permits = Some(Arc::new(Semaphore::new(
            max_batch_streams.saturating_add(max_queued_jobs),
        )));
        Ok(self)
    }

    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    pub fn engine(&self) -> Option<Arc<InferenceEngine>> {
        self.engine.clone()
    }

    pub fn has_loaded_model(&self) -> bool {
        self.engine.is_some()
    }

    pub fn api_key(&self) -> Option<&str> {
        self.api_key.as_deref()
    }

    pub fn token_limits(&self) -> TokenLimits {
        self.token_limits
    }

    pub fn prefix_cache_enabled(&self) -> bool {
        self.prefix_cache_enabled
    }

    pub fn batch_scheduler(&self) -> Option<Arc<BatchScheduler>> {
        self.batch_scheduler.clone()
    }

    pub fn try_acquire_inference_permit(&self) -> Option<OwnedSemaphorePermit> {
        self.inference_permits.clone().try_acquire_owned().ok()
    }

    pub async fn acquire_inference_permit(&self) -> Option<OwnedSemaphorePermit> {
        if self.inference_wait_timeout == Duration::ZERO {
            return self.try_acquire_inference_permit();
        }

        tokio::time::timeout(
            self.inference_wait_timeout,
            self.inference_permits.clone().acquire_owned(),
        )
        .await
        .ok()
        .and_then(Result::ok)
    }

    pub fn try_acquire_batch_admission_permit(&self) -> Option<OwnedSemaphorePermit> {
        self.batch_admission_permits
            .as_ref()?
            .clone()
            .try_acquire_owned()
            .ok()
    }

    pub async fn acquire_batch_admission_permit(&self) -> Option<OwnedSemaphorePermit> {
        let permits = self.batch_admission_permits.as_ref()?.clone();
        if self.inference_wait_timeout == Duration::ZERO {
            return permits.try_acquire_owned().ok();
        }

        tokio::time::timeout(self.inference_wait_timeout, permits.acquire_owned())
            .await
            .ok()
            .and_then(Result::ok)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inference_permit_rejects_second_holder_until_released() {
        let state = ServerState::new("test-model".to_owned());

        let first = state.try_acquire_inference_permit();
        assert!(first.is_some());
        assert!(state.try_acquire_inference_permit().is_none());
        drop(first);
        assert!(state.try_acquire_inference_permit().is_some());
    }

    #[test]
    fn prefix_cache_is_explicitly_enabled() {
        let state = ServerState::new("test-model".to_owned());

        assert!(!state.prefix_cache_enabled());
        assert!(state.with_prefix_cache_enabled(true).prefix_cache_enabled());
    }
}
