use super::{unix_timestamp, usage::Usage};
use crate::runtime::GeneratedText;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct CompletionRequest {
    model: String,
    prompt: String,
    #[serde(default)]
    stream: bool,
    #[serde(default)]
    max_tokens: Option<usize>,
    #[serde(default)]
    suffix: Option<Value>,
    #[serde(default)]
    temperature: Option<Value>,
    #[serde(default)]
    top_p: Option<Value>,
    #[serde(default)]
    n: Option<Value>,
    #[serde(default)]
    logprobs: Option<Value>,
    #[serde(default)]
    echo: Option<Value>,
    #[serde(default)]
    stop: Option<Value>,
    #[serde(default)]
    presence_penalty: Option<Value>,
    #[serde(default)]
    frequency_penalty: Option<Value>,
    #[serde(default)]
    best_of: Option<Value>,
    #[serde(default)]
    logit_bias: Option<Value>,
    #[serde(default)]
    user: Option<Value>,
    #[serde(default, flatten)]
    extra_fields: BTreeMap<String, Value>,
}

impl CompletionRequest {
    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn prompt(&self) -> &str {
        &self.prompt
    }

    pub fn stream(&self) -> bool {
        self.stream
    }

    pub fn max_tokens(&self) -> Option<usize> {
        self.max_tokens
    }

    pub fn unsupported_fields(&self) -> Vec<String> {
        let mut fields = Vec::new();
        if self.suffix.is_some() {
            fields.push("suffix".to_owned());
        }
        if self.temperature.is_some() {
            fields.push("temperature".to_owned());
        }
        if self.top_p.is_some() {
            fields.push("top_p".to_owned());
        }
        if self.n.is_some() {
            fields.push("n".to_owned());
        }
        if self.logprobs.is_some() {
            fields.push("logprobs".to_owned());
        }
        if self.echo.is_some() {
            fields.push("echo".to_owned());
        }
        if self.stop.is_some() {
            fields.push("stop".to_owned());
        }
        if self.presence_penalty.is_some() {
            fields.push("presence_penalty".to_owned());
        }
        if self.frequency_penalty.is_some() {
            fields.push("frequency_penalty".to_owned());
        }
        if self.best_of.is_some() {
            fields.push("best_of".to_owned());
        }
        if self.logit_bias.is_some() {
            fields.push("logit_bias".to_owned());
        }
        if self.user.is_some() {
            fields.push("user".to_owned());
        }
        fields.extend(self.extra_fields.keys().cloned());
        fields
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CompletionResponse {
    id: String,
    object: &'static str,
    created: u64,
    model: String,
    choices: Vec<CompletionChoice>,
    usage: Usage,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CompletionStreamChunk {
    id: String,
    object: &'static str,
    created: u64,
    model: String,
    choices: Vec<CompletionStreamChoice>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletionStreamContext {
    id: String,
    created: u64,
    model: String,
}

impl CompletionStreamContext {
    pub fn new(model: String) -> Self {
        let created = unix_timestamp();
        Self {
            id: format!("cmpl-ferrite-{created}"),
            created,
            model,
        }
    }

    pub fn token(&self, text: String) -> CompletionStreamChunk {
        CompletionStreamChunk {
            id: self.id.clone(),
            object: "text_completion",
            created: self.created,
            model: self.model.clone(),
            choices: vec![CompletionStreamChoice::content(text)],
        }
    }

    pub fn stop(&self) -> CompletionStreamChunk {
        CompletionStreamChunk {
            id: self.id.clone(),
            object: "text_completion",
            created: self.created,
            model: self.model.clone(),
            choices: vec![CompletionStreamChoice::stop()],
        }
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
    finish_reason: Option<&'static str>,
}

impl CompletionStreamChoice {
    fn content(text: String) -> Self {
        Self {
            text,
            index: 0,
            finish_reason: None,
        }
    }

    fn stop() -> Self {
        Self {
            text: String::new(),
            index: 0,
            finish_reason: Some("stop"),
        }
    }
}

impl CompletionResponse {
    pub fn from_generation(model: String, generated: GeneratedText) -> Self {
        let created = unix_timestamp();
        Self {
            id: format!("cmpl-ferrite-{created}"),
            object: "text_completion",
            created,
            model,
            choices: vec![CompletionChoice::new(generated.text().to_owned())],
            usage: Usage::from_generation(&generated),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct CompletionChoice {
    text: String,
    index: usize,
    finish_reason: &'static str,
}

impl CompletionChoice {
    fn new(text: String) -> Self {
        Self {
            text,
            index: 0,
            finish_reason: "stop",
        }
    }
}
