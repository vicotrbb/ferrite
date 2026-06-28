use crate::runtime::InferenceEngine;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct ServerState {
    model_id: String,
    engine: Option<Arc<Mutex<InferenceEngine>>>,
}

impl ServerState {
    pub fn new(model_id: String) -> Self {
        Self {
            model_id,
            engine: None,
        }
    }

    pub fn with_engine(model_id: String, engine: InferenceEngine) -> Self {
        Self {
            model_id,
            engine: Some(Arc::new(Mutex::new(engine))),
        }
    }

    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    pub fn engine(&self) -> Option<Arc<Mutex<InferenceEngine>>> {
        self.engine.clone()
    }
}
