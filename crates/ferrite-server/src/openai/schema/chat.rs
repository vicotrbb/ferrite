use super::{
    chat_message::ChatMessage,
    chat_messages::deserialize_chat_messages,
    function_options::{is_empty_functions, is_neutral_function_call},
    metadata::is_valid_metadata,
    modalities::is_text_only_modalities,
    model_id::deserialize_model_id,
    neutral_options::{is_neutral_bool, is_neutral_number, is_optional_bool},
    prompt_cache_key::is_prompt_cache_key,
    reasoning_effort::is_no_reasoning_effort,
    response_format::{ResponseFormatKind, is_supported_response_format, response_format_kind},
    safety_identifier::is_safety_identifier,
    sampling_options::{SamplingOptionError, sampling_config},
    service_tier::{is_local_service_tier, response_service_tier},
    stop_sequences::{is_supported_stop_sequences, stop_sequences},
    stream_flag::StreamFlag,
    stream_options::StreamOptions,
    token_limit::RequestTokenLimit,
    tool_options::{ToolConfiguration, ToolOptionError},
    unsupported::UnsupportedFields,
    user_identifier::is_user_identifier,
};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ChatCompletionRequest {
    #[serde(default, deserialize_with = "deserialize_model_id")]
    model: String,
    #[serde(default, deserialize_with = "deserialize_chat_messages")]
    messages: Vec<ChatMessage>,
    #[serde(default)]
    stream: StreamFlag,
    #[serde(default)]
    max_tokens: RequestTokenLimit,
    #[serde(default)]
    max_completion_tokens: RequestTokenLimit,
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
    top_k: Option<Value>,
    #[serde(default)]
    top_p: Option<Value>,
    #[serde(default)]
    min_p: Option<Value>,
    #[serde(default)]
    repetition_penalty: Option<Value>,
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
    #[serde(default)]
    return_token_ids: Option<Value>,
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
        self.stream.value()
    }

    pub fn max_tokens(&self) -> Option<usize> {
        self.max_tokens
            .value()
            .or_else(|| self.max_completion_tokens.value())
    }

    pub fn max_tokens_param(&self) -> Option<&'static str> {
        if self.max_tokens.value().is_some() {
            Some("max_tokens")
        } else if self.max_completion_tokens.value().is_some() {
            Some("max_completion_tokens")
        } else {
            None
        }
    }

    pub fn stream_include_usage(&self) -> bool {
        self.stream_options
            .as_ref()
            .is_some_and(StreamOptions::include_usage)
    }

    pub fn stream_include_obfuscation(&self) -> bool {
        self.stream_options
            .as_ref()
            .is_none_or(StreamOptions::include_obfuscation)
    }

    pub fn response_service_tier(&self) -> Option<&'static str> {
        response_service_tier(&self.service_tier)
    }

    pub fn stop_sequences(&self) -> Vec<String> {
        stop_sequences(&self.stop)
    }

    pub(crate) fn requests_json_object(&self) -> bool {
        response_format_kind(&self.response_format) == ResponseFormatKind::JsonObject
    }

    pub(crate) fn tool_configuration(&self) -> Result<ToolConfiguration, ToolOptionError> {
        ToolConfiguration::from_request(&self.tools, &self.tool_choice, &self.parallel_tool_calls)
    }

    pub(crate) fn sampling_config(
        &self,
    ) -> Result<ferrite_inference::sampling::SamplingConfig, SamplingOptionError> {
        sampling_config(
            &self.temperature,
            &self.top_k,
            &self.top_p,
            &self.min_p,
            &self.repetition_penalty,
            &self.frequency_penalty,
            &self.presence_penalty,
            &self.logit_bias,
            &self.seed,
        )
    }

    pub fn cache_options(&self) -> crate::runtime::GenerationCacheOptions {
        crate::runtime::GenerationCacheOptions::from_namespace(
            self.prompt_cache_key
                .as_ref()
                .and_then(Value::as_str)
                .map(str::to_owned),
        )
        .with_prompt_cache_trace_enabled(metadata_flag(&self.metadata, "ferrite_cache_trace"))
    }

    pub fn unsupported_fields(&self) -> Vec<String> {
        let mut fields = UnsupportedFields::new()
            .with_present("functions", !is_empty_functions(&self.functions))
            .with_present(
                "function_call",
                !is_neutral_function_call(&self.function_call, &self.functions),
            )
            .with_present(
                "response_format",
                !is_supported_response_format(&self.response_format),
            )
            .with_present("modalities", !is_text_only_modalities(&self.modalities))
            .with_present("audio", self.audio.is_some())
            .with_present("moderation", self.moderation.is_some())
            .with_present("prediction", self.prediction.is_some())
            .with_present("verbosity", self.verbosity.is_some())
            .with_present("web_search_options", self.web_search_options.is_some())
            .with_present("stream", self.stream.is_malformed())
            .with_present("max_tokens", self.max_tokens.is_malformed())
            .with_present(
                "max_completion_tokens",
                self.max_completion_tokens.is_malformed(),
            )
            .with_present("n", !is_neutral_number(&self.n, 1.0))
            .with_present("stop", !is_supported_stop_sequences(&self.stop))
            .with_present("logprobs", !is_neutral_bool(&self.logprobs, false))
            .with_present("top_logprobs", self.top_logprobs.is_some())
            .with_present("user", !is_user_identifier(&self.user))
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
            .with_present(
                "return_token_ids",
                !is_optional_bool(&self.return_token_ids),
            )
            .with_extra_keys(&self.extra_fields)
            .into_vec();
        if let Some(stream_options) = &self.stream_options {
            if !self.stream() {
                fields.push("stream_options".to_owned());
            } else {
                fields.extend(stream_options.unsupported_request_fields());
            }
        }
        fields.extend(
            self.messages
                .iter()
                .flat_map(ChatMessage::unsupported_fields),
        );
        fields
    }
}

fn metadata_flag(metadata: &Option<Value>, key: &str) -> bool {
    metadata
        .as_ref()
        .and_then(Value::as_object)
        .and_then(|metadata| metadata.get(key))
        .and_then(Value::as_str)
        == Some("true")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_request_maps_prompt_cache_key_to_generation_cache_namespace()
    -> Result<(), Box<dyn std::error::Error>> {
        let request: ChatCompletionRequest = serde_json::from_str(
            r#"{
                "model":"fixture-model",
                "messages":[{"role":"user","content":"hello"}],
                "prompt_cache_key":"tenant-a:thread-1"
            }"#,
        )?;

        assert_eq!(
            request.cache_options().namespace(),
            Some("tenant-a:thread-1")
        );
        Ok(())
    }

    #[test]
    fn chat_request_omits_cache_namespace_for_null_prompt_cache_key()
    -> Result<(), Box<dyn std::error::Error>> {
        let request: ChatCompletionRequest = serde_json::from_str(
            r#"{
                "model":"fixture-model",
                "messages":[{"role":"user","content":"hello"}],
                "prompt_cache_key":null
            }"#,
        )?;

        assert_eq!(request.cache_options().namespace(), None);
        Ok(())
    }

    #[test]
    fn chat_request_metadata_can_enable_ferrite_cache_trace()
    -> Result<(), Box<dyn std::error::Error>> {
        let request: ChatCompletionRequest = serde_json::from_str(
            r#"{
                "model":"fixture-model",
                "messages":[{"role":"user","content":"hello"}],
                "prompt_cache_key":"tenant-a:thread-1",
                "metadata":{"ferrite_cache_trace":"true"}
            }"#,
        )?;

        let cache_options = request.cache_options();

        assert_eq!(cache_options.namespace(), Some("tenant-a:thread-1"));
        assert!(cache_options.prompt_cache_trace_enabled());
        Ok(())
    }

    #[test]
    fn chat_request_keeps_ferrite_cache_trace_disabled_by_default()
    -> Result<(), Box<dyn std::error::Error>> {
        let request: ChatCompletionRequest = serde_json::from_str(
            r#"{
                "model":"fixture-model",
                "messages":[{"role":"user","content":"hello"}],
                "prompt_cache_key":"tenant-a:thread-1"
            }"#,
        )?;

        assert!(!request.cache_options().prompt_cache_trace_enabled());
        Ok(())
    }
}
