use super::{stream_usage::StreamUsage, unix_timestamp, usage::Usage};
use crate::runtime::GeneratedText;
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
    usage: Option<StreamUsage>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatCompletionStreamContext {
    id: String,
    created: u64,
    model: String,
    include_usage: bool,
    service_tier: Option<&'static str>,
}

impl ChatCompletionStreamContext {
    pub fn new(model: String) -> Self {
        let created = unix_timestamp();
        Self {
            id: format!("chatcmpl-ferrite-{created}"),
            created,
            model,
            include_usage: false,
            service_tier: None,
        }
    }

    pub fn with_usage_field(mut self, include_usage: bool) -> Self {
        self.include_usage = include_usage;
        self
    }

    pub fn with_service_tier(mut self, service_tier: Option<&'static str>) -> Self {
        self.service_tier = service_tier;
        self
    }

    pub fn token(&self, content: String) -> ChatCompletionStreamChunk {
        ChatCompletionStreamChunk {
            id: self.id.clone(),
            object: "chat.completion.chunk",
            created: self.created,
            model: self.model.clone(),
            system_fingerprint: None,
            service_tier: self.service_tier,
            choices: vec![ChatCompletionStreamChoice::content(content)],
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
            usage: self.null_usage(),
        }
    }

    pub fn stop(&self) -> ChatCompletionStreamChunk {
        ChatCompletionStreamChunk {
            id: self.id.clone(),
            object: "chat.completion.chunk",
            created: self.created,
            model: self.model.clone(),
            system_fingerprint: None,
            service_tier: self.service_tier,
            choices: vec![ChatCompletionStreamChoice::stop()],
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
            usage: Some(StreamUsage::value(Usage::from_generation(generated))),
        }
    }

    fn null_usage(&self) -> Option<StreamUsage> {
        self.include_usage.then(StreamUsage::null)
    }
}

impl ChatCompletionStreamChunk {
    pub fn from_generation(model: String, generated: &GeneratedText) -> Vec<Self> {
        let context = ChatCompletionStreamContext::new(model);
        let mut chunks = vec![context.role()];
        for text in generated.token_texts() {
            chunks.push(context.token(text.clone()));
        }
        chunks.push(context.stop());
        chunks
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct ChatCompletionStreamChoice {
    index: usize,
    delta: ChatCompletionStreamDelta,
    logprobs: Option<Value>,
    finish_reason: Option<&'static str>,
}

impl ChatCompletionStreamChoice {
    fn role() -> Self {
        Self {
            index: 0,
            delta: ChatCompletionStreamDelta::role(),
            logprobs: None,
            finish_reason: None,
        }
    }

    fn content(content: String) -> Self {
        Self {
            index: 0,
            delta: ChatCompletionStreamDelta::content(content),
            logprobs: None,
            finish_reason: None,
        }
    }

    fn stop() -> Self {
        Self {
            index: 0,
            delta: ChatCompletionStreamDelta::empty(),
            logprobs: None,
            finish_reason: Some("stop"),
        }
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
