use super::{unix_timestamp, usage::Usage};
use crate::runtime::GeneratedText;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct CompletionRequest {
    model: String,
    prompt: String,
    #[serde(default)]
    stream: bool,
    #[serde(default)]
    max_tokens: Option<usize>,
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
