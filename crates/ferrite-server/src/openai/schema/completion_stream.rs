use super::{
    id::response_id, stream_obfuscation::stream_obfuscation, stream_usage::StreamUsage,
    unix_timestamp, usage::Usage,
};
use crate::runtime::{GeneratedText, GenerationFinishReason};
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
    obfuscation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    usage: Option<StreamUsage>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletionStreamContext {
    id: String,
    created: u64,
    model: String,
    include_usage: bool,
    include_obfuscation: bool,
}

impl CompletionStreamContext {
    pub fn new(model: String) -> Self {
        let created = unix_timestamp();
        Self {
            id: response_id("cmpl", created),
            created,
            model,
            include_usage: false,
            include_obfuscation: true,
        }
    }

    pub fn with_usage_field(mut self, include_usage: bool) -> Self {
        self.include_usage = include_usage;
        self
    }

    pub fn with_obfuscation_field(mut self, include_obfuscation: bool) -> Self {
        self.include_obfuscation = include_obfuscation;
        self
    }

    pub fn token(&self, text: String) -> CompletionStreamChunk {
        self.token_chunk(text, None)
    }

    pub fn token_with_ids(&self, text: String, token_ids: &[usize]) -> CompletionStreamChunk {
        self.token_chunk(text, Some(token_ids.to_vec()))
    }

    fn token_chunk(&self, text: String, token_ids: Option<Vec<usize>>) -> CompletionStreamChunk {
        CompletionStreamChunk {
            id: self.id.clone(),
            object: "text_completion",
            created: self.created,
            model: self.model.clone(),
            system_fingerprint: None,
            choices: vec![CompletionStreamChoice::content(text, token_ids)],
            obfuscation: self.obfuscation(),
            usage: self.null_usage(),
        }
    }

    pub fn finish(&self, reason: GenerationFinishReason) -> CompletionStreamChunk {
        CompletionStreamChunk {
            id: self.id.clone(),
            object: "text_completion",
            created: self.created,
            model: self.model.clone(),
            system_fingerprint: None,
            choices: vec![CompletionStreamChoice::finish(reason)],
            obfuscation: self.obfuscation(),
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
            obfuscation: self.obfuscation(),
            usage: Some(StreamUsage::value(Usage::from_generation(generated))),
        }
    }

    fn obfuscation(&self) -> Option<String> {
        self.include_obfuscation.then(stream_obfuscation)
    }

    fn null_usage(&self) -> Option<StreamUsage> {
        self.include_usage.then(StreamUsage::null)
    }
}

impl CompletionStreamChunk {
    pub fn from_generation(model: String, generated: &GeneratedText) -> Vec<Self> {
        let context = CompletionStreamContext::new(model);
        let mut chunks = Vec::new();
        for (index, text) in generated.token_texts().iter().enumerate() {
            let token_ids = generated.token_id_chunks().get(index);
            if let Some(token_ids) = token_ids.filter(|ids| !ids.is_empty()) {
                chunks.push(context.token_with_ids(text.clone(), token_ids));
            } else {
                chunks.push(context.token(text.clone()));
            }
        }
        chunks.push(context.finish(generated.finish_reason()));
        chunks
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct CompletionStreamChoice {
    text: String,
    index: usize,
    logprobs: Option<Value>,
    finish_reason: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    token_ids: Option<Vec<usize>>,
}

impl CompletionStreamChoice {
    fn content(text: String, token_ids: Option<Vec<usize>>) -> Self {
        Self {
            text,
            index: 0,
            logprobs: None,
            finish_reason: None,
            token_ids,
        }
    }

    fn finish(reason: GenerationFinishReason) -> Self {
        Self {
            text: String::new(),
            index: 0,
            logprobs: None,
            finish_reason: Some(finish_reason(reason)),
            token_ids: None,
        }
    }
}

fn finish_reason(reason: GenerationFinishReason) -> &'static str {
    match reason {
        GenerationFinishReason::Stop => "stop",
        GenerationFinishReason::Length => "length",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_stream_context_ids_are_unique_between_streams_in_the_same_second()
    -> Result<(), Box<dyn std::error::Error>> {
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

        assert_eq!(
            context.token("hello".to_owned()).id,
            context.finish(GenerationFinishReason::Stop).id
        );
    }

    #[test]
    fn completion_stream_context_emits_obfuscation_by_default()
    -> Result<(), Box<dyn std::error::Error>> {
        let context = CompletionStreamContext::new("fixture-model".to_owned());
        let chunk = serde_json::to_value(context.token("hello".to_owned()))?;

        assert!(
            chunk["obfuscation"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
        );
        Ok(())
    }

    #[test]
    fn completion_stream_context_can_disable_obfuscation() -> Result<(), Box<dyn std::error::Error>>
    {
        let context =
            CompletionStreamContext::new("fixture-model".to_owned()).with_obfuscation_field(false);
        let chunk = serde_json::to_value(context.token("hello".to_owned()))?;

        assert!(chunk.get("obfuscation").is_none());
        Ok(())
    }

    #[test]
    fn completion_stream_content_chunk_can_include_token_ids()
    -> Result<(), Box<dyn std::error::Error>> {
        let context =
            CompletionStreamContext::new("fixture-model".to_owned()).with_obfuscation_field(false);
        let chunk = serde_json::to_value(context.token_with_ids("hello".to_owned(), &[42, 43]))?;

        assert_eq!(chunk["choices"][0]["text"], "hello");
        assert_eq!(
            chunk["choices"][0]["token_ids"],
            serde_json::json!([42, 43])
        );
        Ok(())
    }
}
