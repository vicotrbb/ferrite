mod catalog;
mod chat;
mod completion;
mod usage;

pub use catalog::{HealthResponse, ModelObject, ModelsResponse};
pub use chat::{ChatCompletionRequest, ChatCompletionResponse, ChatMessage, ChatRole};
pub use completion::{CompletionRequest, CompletionResponse};

use std::time::{SystemTime, UNIX_EPOCH};

fn unix_timestamp() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(_) => 0,
    }
}
