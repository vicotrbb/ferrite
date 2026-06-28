use super::{
    stream_options::StreamOptions, stream_usage::StreamUsage, unix_timestamp,
    unsupported::UnsupportedFields, usage::Usage,
};
use crate::runtime::GeneratedText;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(default)]
    stream: bool,
    #[serde(default)]
    max_tokens: Option<usize>,
    #[serde(default)]
    max_completion_tokens: Option<usize>,
    #[serde(default)]
    tools: Option<Value>,
    #[serde(default)]
    tool_choice: Option<Value>,
    #[serde(default)]
    parallel_tool_calls: Option<Value>,
    #[serde(default)]
    response_format: Option<Value>,
    #[serde(default)]
    temperature: Option<Value>,
    #[serde(default)]
    top_p: Option<Value>,
    #[serde(default)]
    n: Option<Value>,
    #[serde(default)]
    stop: Option<Value>,
    #[serde(default)]
    presence_penalty: Option<Value>,
    #[serde(default)]
    frequency_penalty: Option<Value>,
    #[serde(default)]
    logit_bias: Option<Value>,
    #[serde(default)]
    logprobs: Option<Value>,
    #[serde(default)]
    top_logprobs: Option<Value>,
    #[serde(default)]
    user: Option<Value>,
    #[serde(default)]
    seed: Option<Value>,
    #[serde(default)]
    stream_options: Option<StreamOptions>,
    #[serde(default)]
    store: Option<Value>,
    #[serde(default)]
    metadata: Option<Value>,
    #[serde(default, flatten)]
    extra_fields: BTreeMap<String, Value>,
}

impl ChatCompletionRequest {
    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn messages(&self) -> &[ChatMessage] {
        &self.messages
    }

    pub fn stream(&self) -> bool {
        self.stream
    }

    pub fn max_tokens(&self) -> Option<usize> {
        self.max_tokens.or(self.max_completion_tokens)
    }

    pub fn stream_include_usage(&self) -> bool {
        self.stream_options
            .as_ref()
            .is_some_and(StreamOptions::include_usage)
    }

    pub fn unsupported_fields(&self) -> Vec<String> {
        let mut fields = UnsupportedFields::new()
            .with_present("tools", self.tools.is_some())
            .with_present("tool_choice", self.tool_choice.is_some())
            .with_present("parallel_tool_calls", self.parallel_tool_calls.is_some())
            .with_present("response_format", self.response_format.is_some())
            .with_present("temperature", self.temperature.is_some())
            .with_present("top_p", self.top_p.is_some())
            .with_present("n", self.n.is_some())
            .with_present("stop", self.stop.is_some())
            .with_present("presence_penalty", self.presence_penalty.is_some())
            .with_present("frequency_penalty", self.frequency_penalty.is_some())
            .with_present("logit_bias", self.logit_bias.is_some())
            .with_present("logprobs", self.logprobs.is_some())
            .with_present("top_logprobs", self.top_logprobs.is_some())
            .with_present("user", self.user.is_some())
            .with_present("seed", self.seed.is_some())
            .with_present("store", self.store.is_some())
            .with_present("metadata", self.metadata.is_some())
            .with_extra_keys(&self.extra_fields)
            .into_vec();
        if let Some(stream_options) = &self.stream_options {
            fields.extend(
                stream_options
                    .unsupported_fields()
                    .into_iter()
                    .map(|field| format!("stream_options.{field}")),
            );
        }
        fields
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ChatMessage {
    role: ChatRole,
    content: String,
}

impl ChatMessage {
    #[cfg(test)]
    pub fn new(role: ChatRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }

    pub fn role(&self) -> ChatRole {
        self.role
    }

    pub fn content(&self) -> &str {
        &self.content
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatRole {
    Developer,
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ChatCompletionResponse {
    id: String,
    object: &'static str,
    created: u64,
    model: String,
    choices: Vec<ChatCompletionChoice>,
    usage: Usage,
}

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

impl ChatCompletionResponse {
    pub fn from_generation(model: String, generated: GeneratedText) -> Self {
        let created = unix_timestamp();
        Self {
            id: format!("chatcmpl-ferrite-{created}"),
            object: "chat.completion",
            created,
            model,
            choices: vec![ChatCompletionChoice::new(generated.text().to_owned())],
            usage: Usage::from_generation(&generated),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct ChatCompletionChoice {
    index: usize,
    message: ChatCompletionMessage,
    finish_reason: &'static str,
}

impl ChatCompletionChoice {
    fn new(content: String) -> Self {
        Self {
            index: 0,
            message: ChatCompletionMessage::assistant(content),
            finish_reason: "stop",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct ChatCompletionMessage {
    role: &'static str,
    content: String,
}

impl ChatCompletionMessage {
    fn assistant(content: String) -> Self {
        Self {
            role: "assistant",
            content,
        }
    }
}
