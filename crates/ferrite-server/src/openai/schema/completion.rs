use super::{unix_timestamp, unsupported::UnsupportedFields, usage::Usage};
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
        UnsupportedFields::new()
            .with_present("suffix", self.suffix.is_some())
            .with_present("temperature", self.temperature.is_some())
            .with_present("top_p", self.top_p.is_some())
            .with_present("n", self.n.is_some())
            .with_present("logprobs", self.logprobs.is_some())
            .with_present("echo", self.echo.is_some())
            .with_present("stop", self.stop.is_some())
            .with_present("presence_penalty", self.presence_penalty.is_some())
            .with_present("frequency_penalty", self.frequency_penalty.is_some())
            .with_present("best_of", self.best_of.is_some())
            .with_present("logit_bias", self.logit_bias.is_some())
            .with_present("user", self.user.is_some())
            .with_extra_keys(&self.extra_fields)
            .into_vec()
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
