use super::{
    completion_prompt::CompletionPrompt,
    neutral_options::{is_neutral_bool, is_neutral_number},
    stop_sequences::is_neutral_stop_sequences,
    stream_options::StreamOptions,
    unix_timestamp,
    unsupported::UnsupportedFields,
    usage::Usage,
};
use crate::runtime::GeneratedText;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct CompletionRequest {
    model: String,
    prompt: CompletionPrompt,
    #[serde(default)]
    stream: bool,
    #[serde(default)]
    max_tokens: Option<usize>,
    #[serde(default)]
    stream_options: Option<StreamOptions>,
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

    pub fn prompts(&self) -> &[String] {
        self.prompt.prompts()
    }

    pub fn single_prompt(&self) -> Option<&str> {
        self.prompt.single_prompt()
    }

    pub fn stream(&self) -> bool {
        self.stream
    }

    pub fn max_tokens(&self) -> Option<usize> {
        self.max_tokens
    }

    pub fn stream_include_usage(&self) -> bool {
        self.stream_options
            .as_ref()
            .is_some_and(StreamOptions::include_usage)
    }

    pub fn unsupported_fields(&self) -> Vec<String> {
        let mut fields = UnsupportedFields::new()
            .with_present("suffix", self.suffix.is_some())
            .with_present("temperature", !is_neutral_number(&self.temperature, 0.0))
            .with_present("top_p", !is_neutral_number(&self.top_p, 1.0))
            .with_present("n", !is_neutral_number(&self.n, 1.0))
            .with_present("logprobs", self.logprobs.is_some())
            .with_present("echo", !is_neutral_bool(&self.echo, false))
            .with_present("stop", !is_neutral_stop_sequences(&self.stop))
            .with_present(
                "presence_penalty",
                !is_neutral_number(&self.presence_penalty, 0.0),
            )
            .with_present(
                "frequency_penalty",
                !is_neutral_number(&self.frequency_penalty, 0.0),
            )
            .with_present("best_of", self.best_of.is_some())
            .with_present("logit_bias", self.logit_bias.is_some())
            .with_present("user", self.user.is_some())
            .with_extra_keys(&self.extra_fields)
            .into_vec();
        if let Some(stream_options) = &self.stream_options {
            if !self.stream {
                fields.push("stream_options".to_owned());
            } else {
                fields.extend(
                    stream_options
                        .unsupported_fields()
                        .into_iter()
                        .map(|field| format!("stream_options.{field}")),
                );
            }
        }
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

impl CompletionResponse {
    pub fn from_generation(model: String, generated: GeneratedText) -> Self {
        Self::from_generations(model, vec![generated])
    }

    pub fn from_generations(model: String, generated: Vec<GeneratedText>) -> Self {
        let created = unix_timestamp();
        Self {
            id: format!("cmpl-ferrite-{created}"),
            object: "text_completion",
            created,
            model,
            choices: generated
                .iter()
                .enumerate()
                .map(|(index, generated)| CompletionChoice::new(index, generated.text().to_owned()))
                .collect(),
            usage: Usage::from_generations(&generated),
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
    fn new(index: usize, text: String) -> Self {
        Self {
            text,
            index,
            finish_reason: "stop",
        }
    }
}
