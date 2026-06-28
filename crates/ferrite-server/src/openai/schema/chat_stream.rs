use super::{stream_usage::StreamUsage, unix_timestamp, usage::Usage};
use crate::runtime::GeneratedText;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ChatCompletionStreamChunk {
    id: String,
    object: &'static str,
    created: u64,
    model: String,
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
}

impl ChatCompletionStreamContext {
    pub fn new(model: String) -> Self {
        let created = unix_timestamp();
        Self {
            id: format!("chatcmpl-ferrite-{created}"),
            created,
            model,
            include_usage: false,
        }
    }

    pub fn with_usage_field(mut self, include_usage: bool) -> Self {
        self.include_usage = include_usage;
        self
    }

    pub fn token(&self, content: String) -> ChatCompletionStreamChunk {
        ChatCompletionStreamChunk {
            id: self.id.clone(),
            object: "chat.completion.chunk",
            created: self.created,
            model: self.model.clone(),
            choices: vec![ChatCompletionStreamChoice::content(content)],
            usage: self.null_usage(),
        }
    }

    pub fn stop(&self) -> ChatCompletionStreamChunk {
        ChatCompletionStreamChunk {
            id: self.id.clone(),
            object: "chat.completion.chunk",
            created: self.created,
            model: self.model.clone(),
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
        let mut chunks = Vec::new();
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
    finish_reason: Option<&'static str>,
}

impl ChatCompletionStreamChoice {
    fn content(content: String) -> Self {
        Self {
            index: 0,
            delta: ChatCompletionStreamDelta::content(content),
            finish_reason: None,
        }
    }

    fn stop() -> Self {
        Self {
            index: 0,
            delta: ChatCompletionStreamDelta::empty(),
            finish_reason: Some("stop"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct ChatCompletionStreamDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
}

impl ChatCompletionStreamDelta {
    fn content(content: String) -> Self {
        Self {
            content: Some(content),
        }
    }

    fn empty() -> Self {
        Self { content: None }
    }
}
