mod catalog;
mod chat;
mod chat_content;
mod chat_message;
mod chat_messages;
mod chat_response;
mod chat_stream;
mod completion;
mod completion_prompt;
mod completion_response;
mod completion_stream;
mod function_options;
mod id;
mod logit_bias;
mod message_metadata;
mod metadata;
mod modalities;
mod model_id;
mod neutral_options;
mod prompt_cache_key;
mod reasoning_effort;
mod response_format;
mod safety_identifier;
mod seed;
mod service_tier;
mod stop_sequences;
mod stream_flag;
mod stream_obfuscation;
mod stream_options;
mod stream_usage;
mod token_limit;
mod tool_options;
mod unsupported;
mod usage;
mod user_identifier;

pub use catalog::{HealthResponse, ModelObject, ModelsResponse};
pub use chat::ChatCompletionRequest;
pub use chat_message::{ChatMessage, ChatRole};
pub use chat_response::ChatCompletionResponse;
pub use chat_stream::{ChatCompletionStreamChunk, ChatCompletionStreamContext};
pub use completion::CompletionRequest;
pub use completion_response::CompletionResponse;
pub use completion_stream::{CompletionStreamChunk, CompletionStreamContext};

use std::time::{SystemTime, UNIX_EPOCH};

fn unix_timestamp() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(_) => 0,
    }
}
