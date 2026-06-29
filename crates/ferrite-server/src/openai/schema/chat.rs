use super::{
    chat_content::ChatContent,
    function_options::{is_empty_functions, is_no_function_call},
    metadata::is_valid_metadata,
    modalities::is_text_only_modalities,
    neutral_options::{is_neutral_bool, is_neutral_number},
    prompt_cache_key::is_prompt_cache_key,
    reasoning_effort::is_no_reasoning_effort,
    response_format::is_neutral_response_format,
    safety_identifier::is_safety_identifier,
    seed::is_seed,
    service_tier::{is_local_service_tier, response_service_tier},
    stop_sequences::is_neutral_stop_sequences,
    stream_options::StreamOptions,
    tool_options::{is_disabled_parallel_tool_calls, is_empty_tools, is_no_tool_choice},
    unix_timestamp,
    unsupported::UnsupportedFields,
    usage::Usage,
    user_identifier::is_user_identifier,
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
    functions: Option<Value>,
    #[serde(default)]
    function_call: Option<Value>,
    #[serde(default)]
    response_format: Option<Value>,
    #[serde(default)]
    modalities: Option<Value>,
    #[serde(default)]
    audio: Option<Value>,
    #[serde(default)]
    moderation: Option<Value>,
    #[serde(default)]
    prediction: Option<Value>,
    #[serde(default)]
    verbosity: Option<Value>,
    #[serde(default)]
    web_search_options: Option<Value>,
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
    #[serde(default)]
    prompt_cache_key: Option<Value>,
    #[serde(default)]
    safety_identifier: Option<Value>,
    #[serde(default)]
    reasoning_effort: Option<Value>,
    #[serde(default)]
    service_tier: Option<Value>,
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

    pub fn response_service_tier(&self) -> Option<&'static str> {
        response_service_tier(&self.service_tier)
    }

    pub fn unsupported_fields(&self) -> Vec<String> {
        let mut fields = UnsupportedFields::new()
            .with_present("tools", !is_empty_tools(&self.tools))
            .with_present("tool_choice", !is_no_tool_choice(&self.tool_choice))
            .with_present(
                "parallel_tool_calls",
                !is_disabled_parallel_tool_calls(&self.parallel_tool_calls),
            )
            .with_present("functions", !is_empty_functions(&self.functions))
            .with_present("function_call", !is_no_function_call(&self.function_call))
            .with_present(
                "response_format",
                !is_neutral_response_format(&self.response_format),
            )
            .with_present("modalities", !is_text_only_modalities(&self.modalities))
            .with_present("audio", self.audio.is_some())
            .with_present("moderation", self.moderation.is_some())
            .with_present("prediction", self.prediction.is_some())
            .with_present("verbosity", self.verbosity.is_some())
            .with_present("web_search_options", self.web_search_options.is_some())
            .with_present("temperature", !is_neutral_number(&self.temperature, 0.0))
            .with_present("top_p", !is_neutral_number(&self.top_p, 1.0))
            .with_present("n", !is_neutral_number(&self.n, 1.0))
            .with_present("stop", !is_neutral_stop_sequences(&self.stop))
            .with_present(
                "presence_penalty",
                !is_neutral_number(&self.presence_penalty, 0.0),
            )
            .with_present(
                "frequency_penalty",
                !is_neutral_number(&self.frequency_penalty, 0.0),
            )
            .with_present("logit_bias", self.logit_bias.is_some())
            .with_present("logprobs", !is_neutral_bool(&self.logprobs, false))
            .with_present("top_logprobs", self.top_logprobs.is_some())
            .with_present("user", !is_user_identifier(&self.user))
            .with_present("seed", !is_seed(&self.seed))
            .with_present("store", !is_neutral_bool(&self.store, false))
            .with_present("metadata", !is_valid_metadata(&self.metadata))
            .with_present(
                "prompt_cache_key",
                !is_prompt_cache_key(&self.prompt_cache_key),
            )
            .with_present(
                "safety_identifier",
                !is_safety_identifier(&self.safety_identifier),
            )
            .with_present(
                "reasoning_effort",
                !is_no_reasoning_effort(&self.reasoning_effort),
            )
            .with_present("service_tier", !is_local_service_tier(&self.service_tier))
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

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ChatMessage {
    role: ChatRole,
    content: ChatContent,
}

impl ChatMessage {
    #[cfg(test)]
    pub fn new(role: ChatRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: ChatContent::from_text(content),
        }
    }

    pub fn role(&self) -> ChatRole {
        self.role
    }

    pub fn content(&self) -> &str {
        self.content.text()
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
    system_fingerprint: Option<String>,
    choices: Vec<ChatCompletionChoice>,
    usage: Usage,
    #[serde(skip_serializing_if = "Option::is_none")]
    service_tier: Option<&'static str>,
}

impl ChatCompletionResponse {
    pub fn from_generation(
        model: String,
        generated: GeneratedText,
        service_tier: Option<&'static str>,
    ) -> Self {
        let created = unix_timestamp();
        Self {
            id: format!("chatcmpl-ferrite-{created}"),
            object: "chat.completion",
            created,
            model,
            system_fingerprint: None,
            choices: vec![ChatCompletionChoice::new(generated.text().to_owned())],
            usage: Usage::from_generation(&generated),
            service_tier,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct ChatCompletionChoice {
    index: usize,
    message: ChatCompletionMessage,
    logprobs: Option<Value>,
    finish_reason: &'static str,
}

impl ChatCompletionChoice {
    fn new(content: String) -> Self {
        Self {
            index: 0,
            message: ChatCompletionMessage::assistant(content),
            logprobs: None,
            finish_reason: "stop",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct ChatCompletionMessage {
    role: &'static str,
    content: String,
    refusal: Option<String>,
    annotations: Vec<Value>,
}

impl ChatCompletionMessage {
    fn assistant(content: String) -> Self {
        Self {
            role: "assistant",
            content,
            refusal: None,
            annotations: Vec::new(),
        }
    }
}
