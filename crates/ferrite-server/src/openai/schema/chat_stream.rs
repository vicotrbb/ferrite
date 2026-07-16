use super::{
    id::response_id, stream_obfuscation::stream_obfuscation, stream_usage::StreamUsage,
    unix_timestamp, usage::Usage,
};
use crate::runtime::{GeneratedText, GenerationFinishReason};
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ChatCompletionStreamChunk {
    id: String,
    object: &'static str,
    created: u64,
    model: String,
    system_fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    service_tier: Option<&'static str>,
    choices: Vec<ChatCompletionStreamChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    obfuscation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    usage: Option<StreamUsage>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatCompletionStreamContext {
    id: String,
    created: u64,
    model: String,
    include_usage: bool,
    include_obfuscation: bool,
    service_tier: Option<&'static str>,
}

impl ChatCompletionStreamContext {
    pub fn new(model: String) -> Self {
        let created = unix_timestamp();
        Self {
            id: response_id("chatcmpl", created),
            created,
            model,
            include_usage: false,
            include_obfuscation: true,
            service_tier: None,
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

    pub fn with_service_tier(mut self, service_tier: Option<&'static str>) -> Self {
        self.service_tier = service_tier;
        self
    }

    pub fn token(&self, content: String) -> ChatCompletionStreamChunk {
        self.token_chunk(content, None)
    }

    pub fn token_with_ids(
        &self,
        content: String,
        token_ids: &[usize],
    ) -> ChatCompletionStreamChunk {
        self.token_chunk(content, Some(token_ids.to_vec()))
    }

    fn token_chunk(
        &self,
        content: String,
        token_ids: Option<Vec<usize>>,
    ) -> ChatCompletionStreamChunk {
        ChatCompletionStreamChunk {
            id: self.id.clone(),
            object: "chat.completion.chunk",
            created: self.created,
            model: self.model.clone(),
            system_fingerprint: None,
            service_tier: self.service_tier,
            choices: vec![ChatCompletionStreamChoice::content(content, token_ids)],
            obfuscation: self.obfuscation(),
            usage: self.null_usage(),
        }
    }

    pub fn role(&self) -> ChatCompletionStreamChunk {
        ChatCompletionStreamChunk {
            id: self.id.clone(),
            object: "chat.completion.chunk",
            created: self.created,
            model: self.model.clone(),
            system_fingerprint: None,
            service_tier: self.service_tier,
            choices: vec![ChatCompletionStreamChoice::role()],
            obfuscation: self.obfuscation(),
            usage: self.null_usage(),
        }
    }

    pub fn finish(&self, reason: GenerationFinishReason) -> ChatCompletionStreamChunk {
        ChatCompletionStreamChunk {
            id: self.id.clone(),
            object: "chat.completion.chunk",
            created: self.created,
            model: self.model.clone(),
            system_fingerprint: None,
            service_tier: self.service_tier,
            choices: vec![ChatCompletionStreamChoice::finish(reason)],
            obfuscation: self.obfuscation(),
            usage: self.null_usage(),
        }
    }

    pub fn usage(&self, generated: &GeneratedText) -> ChatCompletionStreamChunk {
        ChatCompletionStreamChunk {
            id: self.id.clone(),
            object: "chat.completion.chunk",
            created: self.created,
            model: self.model.clone(),
            system_fingerprint: None,
            service_tier: self.service_tier,
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

impl ChatCompletionStreamChunk {
    pub fn from_generation(model: String, generated: &GeneratedText) -> Vec<Self> {
        let context = ChatCompletionStreamContext::new(model);
        let mut chunks = vec![context.role()];
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
struct ChatCompletionStreamChoice {
    index: usize,
    delta: ChatCompletionStreamDelta,
    logprobs: Option<Value>,
    finish_reason: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    token_ids: Option<Vec<usize>>,
}

impl ChatCompletionStreamChoice {
    fn role() -> Self {
        Self {
            index: 0,
            delta: ChatCompletionStreamDelta::role(),
            logprobs: None,
            finish_reason: None,
            token_ids: None,
        }
    }

    fn content(content: String, token_ids: Option<Vec<usize>>) -> Self {
        Self {
            index: 0,
            delta: ChatCompletionStreamDelta::content(content),
            logprobs: None,
            finish_reason: None,
            token_ids,
        }
    }

    fn finish(reason: GenerationFinishReason) -> Self {
        Self {
            index: 0,
            delta: ChatCompletionStreamDelta::empty(),
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct ChatCompletionStreamDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
}

impl ChatCompletionStreamDelta {
    fn role() -> Self {
        Self {
            role: Some("assistant"),
            content: Some(String::new()),
        }
    }

    fn content(content: String) -> Self {
        Self {
            role: None,
            content: Some(content),
        }
    }

    fn empty() -> Self {
        Self {
            role: None,
            content: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_stream_context_ids_are_unique_between_streams_in_the_same_second()
    -> Result<(), Box<dyn std::error::Error>> {
        for _ in 0..1_000 {
            let first = ChatCompletionStreamContext::new("fixture-model".to_owned());
            let second = ChatCompletionStreamContext::new("fixture-model".to_owned());

            if first.created == second.created {
                assert_ne!(first.id, second.id);
                return Ok(());
            }
        }

        Err("expected to create two chat stream contexts in the same second".into())
    }

    #[test]
    fn chat_stream_chunks_keep_one_id_within_a_stream() {
        let context = ChatCompletionStreamContext::new("fixture-model".to_owned());
        let id = context.role().id;

        assert_eq!(id, context.token("hello".to_owned()).id);
        assert_eq!(id, context.finish(GenerationFinishReason::Stop).id);
    }

    #[test]
    fn chat_stream_context_emits_obfuscation_by_default() -> Result<(), Box<dyn std::error::Error>>
    {
        let context = ChatCompletionStreamContext::new("fixture-model".to_owned());
        let chunk = serde_json::to_value(context.token("hello".to_owned()))?;

        assert!(
            chunk["obfuscation"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
        );
        Ok(())
    }

    #[test]
    fn chat_stream_context_can_disable_obfuscation() -> Result<(), Box<dyn std::error::Error>> {
        let context = ChatCompletionStreamContext::new("fixture-model".to_owned())
            .with_obfuscation_field(false);
        let chunk = serde_json::to_value(context.token("hello".to_owned()))?;

        assert!(chunk.get("obfuscation").is_none());
        Ok(())
    }

    #[test]
    fn chat_stream_content_chunk_can_include_token_ids() -> Result<(), Box<dyn std::error::Error>> {
        let context = ChatCompletionStreamContext::new("fixture-model".to_owned())
            .with_obfuscation_field(false);
        let chunk = serde_json::to_value(context.token_with_ids("hello".to_owned(), &[42, 43]))?;

        assert_eq!(chunk["choices"][0]["delta"]["content"], "hello");
        assert_eq!(
            chunk["choices"][0]["token_ids"],
            serde_json::json!([42, 43])
        );
        Ok(())
    }
}
