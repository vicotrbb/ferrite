use super::{
    completion_prompt::CompletionPrompt,
    id::response_id,
    logit_bias::is_neutral_logit_bias,
    model_id::deserialize_model_id,
    neutral_options::{is_neutral_number, is_neutral_number_in},
    seed::is_seed,
    stop_sequences::{is_supported_stop_sequences, stop_sequences},
    stream_flag::StreamFlag,
    stream_options::StreamOptions,
    token_limit::RequestTokenLimit,
    unix_timestamp,
    unsupported::UnsupportedFields,
    usage::Usage,
    user_identifier::is_user_identifier,
};
use crate::runtime::{GeneratedText, GenerationFinishReason};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct CompletionRequest {
    #[serde(default, deserialize_with = "deserialize_model_id")]
    model: String,
    #[serde(default)]
    prompt: CompletionPrompt,
    #[serde(default)]
    stream: StreamFlag,
    #[serde(default)]
    max_tokens: RequestTokenLimit,
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
    #[serde(default)]
    seed: Option<Value>,
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
        self.stream.value()
    }

    pub fn max_tokens(&self) -> Option<usize> {
        self.max_tokens.value()
    }

    pub fn max_tokens_param(&self) -> Option<&'static str> {
        self.max_tokens.value().map(|_| "max_tokens")
    }

    pub fn stream_include_usage(&self) -> bool {
        self.stream_options
            .as_ref()
            .is_some_and(StreamOptions::include_usage)
    }

    pub fn echo(&self) -> bool {
        self.echo.as_ref().is_some_and(|value| value == true)
    }

    pub fn stop_sequences(&self) -> Vec<String> {
        stop_sequences(&self.stop)
    }

    pub fn unsupported_fields(&self) -> Vec<String> {
        let mut fields = UnsupportedFields::new()
            .with_present("prompt", self.prompt.has_unsupported_form())
            .with_present("stream", self.stream.is_malformed())
            .with_present("max_tokens", self.max_tokens.is_malformed())
            .with_present("suffix", self.suffix.is_some())
            .with_present(
                "temperature",
                !is_neutral_number_in(&self.temperature, &[0.0, 1.0]),
            )
            .with_present("top_p", !is_neutral_number(&self.top_p, 1.0))
            .with_present("n", !is_neutral_number(&self.n, 1.0))
            .with_present("logprobs", self.logprobs.is_some())
            .with_present("echo", !self.echo_option_is_supported())
            .with_present("stop", !is_supported_stop_sequences(&self.stop))
            .with_present(
                "presence_penalty",
                !is_neutral_number(&self.presence_penalty, 0.0),
            )
            .with_present(
                "frequency_penalty",
                !is_neutral_number(&self.frequency_penalty, 0.0),
            )
            .with_present("best_of", !is_neutral_number(&self.best_of, 1.0))
            .with_present("logit_bias", !is_neutral_logit_bias(&self.logit_bias))
            .with_present("user", !is_user_identifier(&self.user))
            .with_present("seed", !is_seed(&self.seed))
            .with_extra_keys(&self.extra_fields)
            .into_vec();
        if let Some(stream_options) = &self.stream_options {
            if !self.stream() {
                fields.push("stream_options".to_owned());
            } else {
                fields.extend(stream_options.unsupported_request_fields());
            }
        }
        fields
    }

    fn echo_option_is_supported(&self) -> bool {
        match &self.echo {
            None => true,
            Some(Value::Bool(false)) => true,
            Some(Value::Bool(true)) => !self.stream(),
            Some(_) => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CompletionResponse {
    id: String,
    object: &'static str,
    created: u64,
    model: String,
    system_fingerprint: Option<String>,
    choices: Vec<CompletionChoice>,
    usage: Usage,
}

impl CompletionResponse {
    pub fn from_generation(model: String, generated: GeneratedText) -> Self {
        Self::from_generations(model, vec![generated])
    }

    pub fn from_generations(model: String, generated: Vec<GeneratedText>) -> Self {
        Self::from_prompt_generations(model, &[], generated, false)
    }

    pub fn from_prompt_generations(
        model: String,
        prompts: &[String],
        generated: Vec<GeneratedText>,
        echo: bool,
    ) -> Self {
        let created = unix_timestamp();
        Self {
            id: response_id("cmpl", created),
            object: "text_completion",
            created,
            model,
            system_fingerprint: None,
            choices: generated
                .iter()
                .enumerate()
                .map(|(index, generated)| {
                    let prompt = echo.then(|| prompts.get(index).map(String::as_str).unwrap_or(""));
                    CompletionChoice::new(index, prompt, generated)
                })
                .collect(),
            usage: Usage::from_generations(&generated),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct CompletionChoice {
    text: String,
    index: usize,
    logprobs: Option<Value>,
    finish_reason: &'static str,
}

impl CompletionChoice {
    fn new(index: usize, prompt: Option<&str>, generated: &GeneratedText) -> Self {
        let text = match prompt {
            Some(prompt) => format!("{prompt}{}", generated.text()),
            None => generated.text().to_owned(),
        };
        Self {
            text,
            index,
            logprobs: None,
            finish_reason: finish_reason(generated.finish_reason()),
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
    fn completion_response_ids_are_unique_within_the_same_second(
    ) -> Result<(), Box<dyn std::error::Error>> {
        for _ in 0..1_000 {
            let first =
                CompletionResponse::from_generation("fixture-model".to_owned(), generated());
            let second =
                CompletionResponse::from_generation("fixture-model".to_owned(), generated());

            if first.created == second.created {
                assert_ne!(first.id, second.id);
                return Ok(());
            }
        }

        Err("expected to create two completion responses in the same second".into())
    }

    fn generated() -> GeneratedText {
        GeneratedText::new("winner".to_owned(), 1, 1, vec!["winner".to_owned()])
    }
}
