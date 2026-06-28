mod catalog;
mod chat;
mod chat_stream;
mod completion;
mod stream_options;
mod stream_usage;
mod unsupported;
mod usage;

pub use catalog::{HealthResponse, ModelObject, ModelsResponse};
pub use chat::{ChatCompletionRequest, ChatCompletionResponse, ChatMessage, ChatRole};
pub use chat_stream::{ChatCompletionStreamChunk, ChatCompletionStreamContext};
pub use completion::{
    CompletionRequest, CompletionResponse, CompletionStreamChunk, CompletionStreamContext,
};

use std::time::{SystemTime, UNIX_EPOCH};

fn unix_timestamp() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(_) => 0,
    }
}
