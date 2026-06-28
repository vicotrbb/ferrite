use crate::limits::TokenLimits;
use crate::runtime::InferenceEngine;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

const INFERENCE_PERMITS: usize = 1;

#[derive(Clone, Debug)]
pub struct ServerState {
    model_id: String,
    engine: Option<Arc<Mutex<InferenceEngine>>>,
    inference_permits: Arc<Semaphore>,
    inference_wait_timeout: Duration,
    api_key: Option<Arc<str>>,
    token_limits: TokenLimits,
}

impl ServerState {
    pub fn new(model_id: String) -> Self {
        Self {
            model_id,
            engine: None,
            inference_permits: Arc::new(Semaphore::new(INFERENCE_PERMITS)),
            inference_wait_timeout: Duration::ZERO,
            api_key: None,
            token_limits: TokenLimits::default(),
        }
    }

    pub fn with_engine(model_id: String, engine: InferenceEngine) -> Self {
        Self {
            model_id,
            engine: Some(Arc::new(Mutex::new(engine))),
            inference_permits: Arc::new(Semaphore::new(INFERENCE_PERMITS)),
            inference_wait_timeout: Duration::ZERO,
            api_key: None,
            token_limits: TokenLimits::default(),
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

    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    pub fn engine(&self) -> Option<Arc<Mutex<InferenceEngine>>> {
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
}
