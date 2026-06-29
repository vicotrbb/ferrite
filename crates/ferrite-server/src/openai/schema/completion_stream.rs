use super::{id::response_id, stream_usage::StreamUsage, unix_timestamp, usage::Usage};
use crate::runtime::GeneratedText;
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CompletionStreamChunk {
    id: String,
    object: &'static str,
    created: u64,
    model: String,
    system_fingerprint: Option<String>,
    choices: Vec<CompletionStreamChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    usage: Option<StreamUsage>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletionStreamContext {
    id: String,
    created: u64,
    model: String,
    include_usage: bool,
}

impl CompletionStreamContext {
    pub fn new(model: String) -> Self {
        let created = unix_timestamp();
        Self {
            id: response_id("cmpl", created),
            created,
            model,
            include_usage: false,
        }
    }

    pub fn with_usage_field(mut self, include_usage: bool) -> Self {
        self.include_usage = include_usage;
        self
    }

    pub fn token(&self, text: String) -> CompletionStreamChunk {
        CompletionStreamChunk {
            id: self.id.clone(),
            object: "text_completion",
            created: self.created,
            model: self.model.clone(),
            system_fingerprint: None,
            choices: vec![CompletionStreamChoice::content(text)],
            usage: self.null_usage(),
        }
    }

    pub fn stop(&self) -> CompletionStreamChunk {
        CompletionStreamChunk {
            id: self.id.clone(),
            object: "text_completion",
            created: self.created,
            model: self.model.clone(),
            system_fingerprint: None,
            choices: vec![CompletionStreamChoice::stop()],
            usage: self.null_usage(),
        }
    }

    pub fn usage(&self, generated: &GeneratedText) -> CompletionStreamChunk {
        CompletionStreamChunk {
            id: self.id.clone(),
            object: "text_completion",
            created: self.created,
            model: self.model.clone(),
            system_fingerprint: None,
            choices: Vec::new(),
            usage: Some(StreamUsage::value(Usage::from_generation(generated))),
        }
    }

    fn null_usage(&self) -> Option<StreamUsage> {
        self.include_usage.then(StreamUsage::null)
    }
}

impl CompletionStreamChunk {
    pub fn from_generation(model: String, generated: &GeneratedText) -> Vec<Self> {
        let context = CompletionStreamContext::new(model);
        let mut chunks = Vec::new();
        for text in generated.token_texts() {
            chunks.push(context.token(text.clone()));
        }
        chunks.push(context.stop());
        chunks
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct CompletionStreamChoice {
    text: String,
    index: usize,
    logprobs: Option<Value>,
    finish_reason: Option<&'static str>,
}

impl CompletionStreamChoice {
    fn content(text: String) -> Self {
        Self {
            text,
            index: 0,
            logprobs: None,
            finish_reason: None,
        }
    }

    fn stop() -> Self {
        Self {
            text: String::new(),
            index: 0,
            logprobs: None,
            finish_reason: Some("stop"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_stream_context_ids_are_unique_between_streams_in_the_same_second(
    ) -> Result<(), Box<dyn std::error::Error>> {
        for _ in 0..1_000 {
            let first = CompletionStreamContext::new("fixture-model".to_owned());
            let second = CompletionStreamContext::new("fixture-model".to_owned());

            if first.created == second.created {
                assert_ne!(first.id, second.id);
                return Ok(());
            }
        }

        Err("expected to create two completion stream contexts in the same second".into())
    }

    #[test]
    fn completion_stream_chunks_keep_one_id_within_a_stream() {
        let context = CompletionStreamContext::new("fixture-model".to_owned());

        assert_eq!(context.token("hello".to_owned()).id, context.stop().id);
    }
}
