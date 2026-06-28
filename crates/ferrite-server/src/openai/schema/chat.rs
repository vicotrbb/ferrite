use super::{unix_timestamp, usage::Usage};
use crate::runtime::GeneratedText;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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

    pub fn unsupported_fields(&self) -> Vec<&'static str> {
        let mut fields = Vec::new();
        if self.tools.is_some() {
            fields.push("tools");
        }
        if self.tool_choice.is_some() {
            fields.push("tool_choice");
        }
        if self.parallel_tool_calls.is_some() {
            fields.push("parallel_tool_calls");
        }
        if self.response_format.is_some() {
            fields.push("response_format");
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatCompletionStreamContext {
    id: String,
    created: u64,
    model: String,
}

impl ChatCompletionStreamContext {
    pub fn new(model: String) -> Self {
        let created = unix_timestamp();
        Self {
            id: format!("chatcmpl-ferrite-{created}"),
            created,
            model,
        }
    }

    pub fn token(&self, content: String) -> ChatCompletionStreamChunk {
        ChatCompletionStreamChunk {
            id: self.id.clone(),
            object: "chat.completion.chunk",
            created: self.created,
            model: self.model.clone(),
            choices: vec![ChatCompletionStreamChoice::content(content)],
        }
    }

    pub fn stop(&self) -> ChatCompletionStreamChunk {
        ChatCompletionStreamChunk {
            id: self.id.clone(),
            object: "chat.completion.chunk",
            created: self.created,
            model: self.model.clone(),
            choices: vec![ChatCompletionStreamChoice::stop()],
        }
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
