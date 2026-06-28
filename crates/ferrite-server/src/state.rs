use crate::runtime::InferenceEngine;
use std::sync::{Arc, Mutex};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

const INFERENCE_PERMITS: usize = 1;

#[derive(Clone, Debug)]
pub struct ServerState {
    model_id: String,
    engine: Option<Arc<Mutex<InferenceEngine>>>,
    inference_permits: Arc<Semaphore>,
    api_key: Option<Arc<str>>,
}

impl ServerState {
    pub fn new(model_id: String) -> Self {
        Self {
            model_id,
            engine: None,
            inference_permits: Arc::new(Semaphore::new(INFERENCE_PERMITS)),
            api_key: None,
        }
    }

    pub fn with_engine(model_id: String, engine: InferenceEngine) -> Self {
        Self {
            model_id,
            engine: Some(Arc::new(Mutex::new(engine))),
            inference_permits: Arc::new(Semaphore::new(INFERENCE_PERMITS)),
            api_key: None,
        }
    }

    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(Arc::from(api_key.into()));
        self
    }

    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    pub fn engine(&self) -> Option<Arc<Mutex<InferenceEngine>>> {
        self.engine.clone()
    }

    pub fn api_key(&self) -> Option<&str> {
        self.api_key.as_deref()
    }

    pub fn try_acquire_inference_permit(&self) -> Option<OwnedSemaphorePermit> {
        self.inference_permits.clone().try_acquire_owned().ok()
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
