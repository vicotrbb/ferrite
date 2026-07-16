use super::{
    chat_message::{ChatMessage, ChatRole},
    id::response_id,
    metadata::is_valid_metadata,
    model_id::deserialize_model_id,
    neutral_options::{is_neutral_bool, is_optional_bool},
    prompt_cache_key::is_prompt_cache_key,
    reasoning_effort::is_no_reasoning_effort,
    safety_identifier::is_safety_identifier,
    sampling_options::{SamplingOptionError, sampling_config},
    service_tier::{is_local_service_tier, response_service_tier},
    stream_flag::StreamFlag,
    token_limit::RequestTokenLimit,
    unix_timestamp,
    unsupported::UnsupportedFields,
    user_identifier::is_user_identifier,
};
use crate::runtime::{GeneratedText, GenerationCacheOptions, GenerationFinishReason};
use ferrite_inference::sampling::SamplingConfig;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fmt;

const MAX_INPUT_MESSAGES: usize = 256;
const MAX_CONTENT_PARTS: usize = 256;
const MAX_INPUT_TEXT_BYTES: usize = 1024 * 1024;

/// The bounded, non-streaming subset of `POST /v1/responses` supported locally.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ResponsesRequest {
    #[serde(default, deserialize_with = "deserialize_model_id")]
    model: String,
    #[serde(default)]
    input: Option<Value>,
    #[serde(default)]
    instructions: Option<Value>,
    #[serde(default)]
    stream: StreamFlag,
    #[serde(default)]
    max_output_tokens: RequestTokenLimit,
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
    frequency_penalty: Option<Value>,
    #[serde(default)]
    presence_penalty: Option<Value>,
    #[serde(default)]
    logit_bias: Option<Value>,
    #[serde(default)]
    seed: Option<Value>,
    #[serde(default)]
    background: Option<Value>,
    #[serde(default)]
    store: Option<Value>,
    #[serde(default)]
    metadata: Option<Value>,
    #[serde(default)]
    previous_response_id: Option<Value>,
    #[serde(default)]
    include: Option<Value>,
    #[serde(default)]
    tools: Option<Value>,
    #[serde(default)]
    tool_choice: Option<Value>,
    #[serde(default)]
    parallel_tool_calls: Option<Value>,
    #[serde(default)]
    text: Option<Value>,
    #[serde(default)]
    reasoning: Option<Value>,
    #[serde(default)]
    truncation: Option<Value>,
    #[serde(default)]
    user: Option<Value>,
    #[serde(default)]
    prompt_cache_key: Option<Value>,
    #[serde(default)]
    safety_identifier: Option<Value>,
    #[serde(default)]
    service_tier: Option<Value>,
    #[serde(default)]
    max_tool_calls: Option<Value>,
    #[serde(default, flatten)]
    extra_fields: BTreeMap<String, Value>,
}

impl ResponsesRequest {
    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn stream(&self) -> bool {
        self.stream.value()
    }

    pub fn max_output_tokens(&self) -> Option<usize> {
        self.max_output_tokens.value()
    }

    pub fn messages(&self) -> Result<Vec<ChatMessage>, ResponsesRequestError> {
        let mut messages = Vec::new();
        let mut text_bytes = 0;
        if let Some(instructions) = self.instructions_text()? {
            add_text_bytes(&mut text_bytes, instructions.len())?;
            messages.push(ChatMessage::new(ChatRole::System, instructions));
        }

        match self.input.as_ref() {
            Some(Value::String(input)) => {
                add_nonempty_text(&mut text_bytes, input)?;
                messages.push(ChatMessage::new(ChatRole::User, input));
            }
            Some(Value::Array(items)) => {
                if items.is_empty() {
                    return Err(ResponsesRequestError::input(
                        "input must contain at least one message",
                    ));
                }
                if items.len() > MAX_INPUT_MESSAGES {
                    return Err(ResponsesRequestError::input(format!(
                        "input cannot contain more than {MAX_INPUT_MESSAGES} messages"
                    )));
                }
                for item in items {
                    messages.push(parse_input_message(item, &mut text_bytes)?);
                }
            }
            Some(Value::Null) | None => {
                return Err(ResponsesRequestError::input("input is required"));
            }
            Some(_) => {
                return Err(ResponsesRequestError::input(
                    "input must be a string or an array of message objects",
                ));
            }
        }
        Ok(messages)
    }

    pub(crate) fn sampling_config(&self) -> Result<SamplingConfig, SamplingOptionError> {
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

    pub fn cache_options(&self) -> GenerationCacheOptions {
        GenerationCacheOptions::from_namespace(
            self.prompt_cache_key
                .as_ref()
                .and_then(Value::as_str)
                .map(str::to_owned),
        )
        .with_prompt_cache_trace_enabled(metadata_flag(&self.metadata, "ferrite_cache_trace"))
    }

    pub fn unsupported_fields(&self) -> Vec<String> {
        UnsupportedFields::new()
            .with_present("stream", self.stream.is_malformed())
            .with_present("max_output_tokens", self.max_output_tokens.is_malformed())
            .with_present("background", !is_neutral_bool(&self.background, false))
            .with_present("store", !is_neutral_bool(&self.store, false))
            .with_present("metadata", !is_valid_metadata(&self.metadata))
            .with_present(
                "previous_response_id",
                !is_missing_or_null(&self.previous_response_id),
            )
            .with_present("include", !is_missing_null_or_empty_array(&self.include))
            .with_present("tools", !is_missing_null_or_empty_array(&self.tools))
            .with_present("tool_choice", !is_neutral_tool_choice(&self.tool_choice))
            .with_present(
                "parallel_tool_calls",
                !is_optional_bool(&self.parallel_tool_calls),
            )
            .with_present("text", !is_plain_text_config(&self.text))
            .with_present("reasoning", !is_no_reasoning_config(&self.reasoning))
            .with_present("truncation", !is_disabled_truncation(&self.truncation))
            .with_present("user", !is_user_identifier(&self.user))
            .with_present(
                "prompt_cache_key",
                !is_prompt_cache_key(&self.prompt_cache_key),
            )
            .with_present(
                "safety_identifier",
                !is_safety_identifier(&self.safety_identifier),
            )
            .with_present("service_tier", !is_local_service_tier(&self.service_tier))
            .with_present("max_tool_calls", !is_missing_or_null(&self.max_tool_calls))
            .with_extra_keys(&self.extra_fields)
            .into_vec()
    }

    pub fn response_service_tier(&self) -> Option<&'static str> {
        response_service_tier(&self.service_tier)
    }

    fn instructions_text(&self) -> Result<Option<&str>, ResponsesRequestError> {
        match self.instructions.as_ref() {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(instructions)) => Ok(Some(instructions)),
            Some(_) => Err(ResponsesRequestError::new(
                "instructions",
                "instructions must be a string or null",
            )),
        }
    }

    fn instructions_for_response(&self) -> Option<String> {
        self.instructions
            .as_ref()
            .and_then(Value::as_str)
            .map(str::to_owned)
    }

    fn response_metadata(&self) -> Map<String, Value> {
        self.metadata
            .as_ref()
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default()
    }

    fn response_parallel_tool_calls(&self) -> bool {
        self.parallel_tool_calls
            .as_ref()
            .and_then(Value::as_bool)
            .unwrap_or(true)
    }

    fn response_tool_choice(&self) -> &'static str {
        match self.tool_choice.as_ref().and_then(Value::as_str) {
            Some("none") => "none",
            _ => "auto",
        }
    }

    fn response_prompt_cache_key(&self) -> Option<String> {
        self.prompt_cache_key
            .as_ref()
            .and_then(Value::as_str)
            .map(str::to_owned)
    }

    fn response_safety_identifier(&self) -> Option<String> {
        self.safety_identifier
            .as_ref()
            .and_then(Value::as_str)
            .map(str::to_owned)
    }
}

fn parse_input_message(
    value: &Value,
    text_bytes: &mut usize,
) -> Result<ChatMessage, ResponsesRequestError> {
    let object = value
        .as_object()
        .ok_or_else(|| ResponsesRequestError::input("input array items must be message objects"))?;
    if object
        .keys()
        .any(|key| !matches!(key.as_str(), "type" | "role" | "content"))
    {
        return Err(ResponsesRequestError::input(
            "input message objects only support type, role, and content",
        ));
    }
    if object
        .get("type")
        .is_some_and(|value| value.as_str() != Some("message"))
    {
        return Err(ResponsesRequestError::input(
            "input message type must be message",
        ));
    }
    let role = match object.get("role").and_then(Value::as_str) {
        Some("developer") => ChatRole::Developer,
        Some("system") => ChatRole::System,
        Some("user") => ChatRole::User,
        Some("assistant") => ChatRole::Assistant,
        _ => {
            return Err(ResponsesRequestError::input(
                "input message role must be developer, system, user, or assistant",
            ));
        }
    };
    let content = parse_input_content(object.get("content"), role, text_bytes)?;
    Ok(ChatMessage::new(role, content))
}

fn parse_input_content(
    value: Option<&Value>,
    role: ChatRole,
    text_bytes: &mut usize,
) -> Result<String, ResponsesRequestError> {
    match value {
        Some(Value::String(text)) => {
            add_nonempty_text(text_bytes, text)?;
            Ok(text.clone())
        }
        Some(Value::Array(parts)) => {
            if parts.is_empty() {
                return Err(ResponsesRequestError::input(
                    "input message content must contain at least one text part",
                ));
            }
            if parts.len() > MAX_CONTENT_PARTS {
                return Err(ResponsesRequestError::input(format!(
                    "input message content cannot contain more than {MAX_CONTENT_PARTS} parts"
                )));
            }
            let mut content = String::new();
            for part in parts {
                let object = part.as_object().ok_or_else(|| {
                    ResponsesRequestError::input("input content parts must be objects")
                })?;
                if object
                    .keys()
                    .any(|key| !matches!(key.as_str(), "type" | "text"))
                {
                    return Err(ResponsesRequestError::input(
                        "input content parts only support type and text",
                    ));
                }
                let expected_type = if role == ChatRole::Assistant {
                    "output_text"
                } else {
                    "input_text"
                };
                if object.get("type").and_then(Value::as_str) != Some(expected_type) {
                    return Err(ResponsesRequestError::input(format!(
                        "{} message content parts must use type {expected_type}",
                        role_name(role)
                    )));
                }
                let text = object.get("text").and_then(Value::as_str).ok_or_else(|| {
                    ResponsesRequestError::input("input content part text must be a string")
                })?;
                add_text_bytes(text_bytes, text.len())?;
                content.push_str(text);
            }
            if content.trim().is_empty() {
                return Err(ResponsesRequestError::input(
                    "input message content must contain non-whitespace text",
                ));
            }
            Ok(content)
        }
        _ => Err(ResponsesRequestError::input(
            "input message content must be a string or an array of text parts",
        )),
    }
}

fn role_name(role: ChatRole) -> &'static str {
    match role {
        ChatRole::Developer => "developer",
        ChatRole::System => "system",
        ChatRole::User => "user",
        ChatRole::Assistant => "assistant",
        ChatRole::Tool | ChatRole::Function | ChatRole::Unknown => "unsupported",
    }
}

fn add_nonempty_text(text_bytes: &mut usize, text: &str) -> Result<(), ResponsesRequestError> {
    if text.trim().is_empty() {
        return Err(ResponsesRequestError::input(
            "input must contain non-whitespace text",
        ));
    }
    add_text_bytes(text_bytes, text.len())
}

fn add_text_bytes(text_bytes: &mut usize, additional: usize) -> Result<(), ResponsesRequestError> {
    *text_bytes = text_bytes
        .checked_add(additional)
        .ok_or_else(|| ResponsesRequestError::input("input text size overflowed"))?;
    if *text_bytes > MAX_INPUT_TEXT_BYTES {
        return Err(ResponsesRequestError::input(format!(
            "input and instructions cannot exceed {MAX_INPUT_TEXT_BYTES} bytes"
        )));
    }
    Ok(())
}

fn is_missing_or_null(value: &Option<Value>) -> bool {
    value.as_ref().is_none_or(Value::is_null)
}

fn is_missing_null_or_empty_array(value: &Option<Value>) -> bool {
    value
        .as_ref()
        .is_none_or(|value| value.is_null() || value.as_array().is_some_and(Vec::is_empty))
}

fn is_neutral_tool_choice(value: &Option<Value>) -> bool {
    value
        .as_ref()
        .is_none_or(|value| value.is_null() || matches!(value.as_str(), Some("auto" | "none")))
}

fn is_plain_text_config(value: &Option<Value>) -> bool {
    let Some(value) = value.as_ref() else {
        return true;
    };
    if value.is_null() {
        return true;
    }
    let Some(object) = value.as_object() else {
        return false;
    };
    if object
        .keys()
        .any(|key| !matches!(key.as_str(), "format" | "verbosity"))
    {
        return false;
    }
    let valid_format = object.get("format").is_none_or(|format| {
        format.is_null()
            || format.as_object().is_some_and(|format| {
                format.len() == 1 && format.get("type").and_then(Value::as_str) == Some("text")
            })
    });
    let valid_verbosity = object
        .get("verbosity")
        .is_none_or(|verbosity| verbosity.is_null() || verbosity.as_str() == Some("medium"));
    valid_format && valid_verbosity
}

fn is_no_reasoning_config(value: &Option<Value>) -> bool {
    let Some(value) = value.as_ref() else {
        return true;
    };
    if value.is_null() {
        return true;
    }
    let Some(object) = value.as_object() else {
        return false;
    };
    object
        .keys()
        .all(|key| matches!(key.as_str(), "effort" | "summary"))
        && is_no_reasoning_effort(&object.get("effort").cloned())
        && object.get("summary").is_none_or(Value::is_null)
}

fn is_disabled_truncation(value: &Option<Value>) -> bool {
    value
        .as_ref()
        .is_none_or(|value| value.is_null() || value.as_str() == Some("disabled"))
}

fn metadata_flag(metadata: &Option<Value>, key: &str) -> bool {
    metadata
        .as_ref()
        .and_then(Value::as_object)
        .and_then(|metadata| metadata.get(key))
        .and_then(Value::as_str)
        == Some("true")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResponsesRequestError {
    parameter: &'static str,
    message: String,
}

impl ResponsesRequestError {
    fn new(parameter: &'static str, message: impl Into<String>) -> Self {
        Self {
            parameter,
            message: message.into(),
        }
    }

    fn input(message: impl Into<String>) -> Self {
        Self::new("input", message)
    }

    pub fn parameter(&self) -> &'static str {
        self.parameter
    }
}

impl fmt::Display for ResponsesRequestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ResponsesRequestError {}

#[derive(Clone, Debug, Serialize)]
pub struct ResponsesResponse {
    id: String,
    object: &'static str,
    created_at: u64,
    status: &'static str,
    completed_at: Option<u64>,
    background: bool,
    conversation: Option<Value>,
    error: Option<Value>,
    incomplete_details: Option<IncompleteDetails>,
    instructions: Option<String>,
    max_output_tokens: Option<usize>,
    max_tool_calls: Option<usize>,
    model: String,
    output: Vec<ResponseOutputMessage>,
    parallel_tool_calls: bool,
    previous_response_id: Option<String>,
    prompt_cache_key: Option<String>,
    reasoning: ResponseReasoning,
    safety_identifier: Option<String>,
    service_tier: &'static str,
    store: bool,
    temperature: f32,
    text: ResponseTextConfiguration,
    tool_choice: &'static str,
    tools: Vec<Value>,
    top_logprobs: usize,
    top_p: f32,
    truncation: &'static str,
    usage: ResponsesUsage,
    user: Option<String>,
    metadata: Map<String, Value>,
}

impl ResponsesResponse {
    pub fn from_generation(
        request: &ResponsesRequest,
        model: String,
        sampling: &SamplingConfig,
        generated: GeneratedText,
    ) -> Self {
        let created_at = unix_timestamp();
        let incomplete = generated.finish_reason() == GenerationFinishReason::Length;
        let status = if incomplete {
            "incomplete"
        } else {
            "completed"
        };
        Self {
            id: response_id("resp", created_at),
            object: "response",
            created_at,
            status,
            completed_at: (!incomplete).then_some(created_at),
            background: false,
            conversation: None,
            error: None,
            incomplete_details: incomplete.then_some(IncompleteDetails {
                reason: "max_output_tokens",
            }),
            instructions: request.instructions_for_response(),
            max_output_tokens: request.max_output_tokens(),
            max_tool_calls: None,
            model,
            output: vec![ResponseOutputMessage::new(&generated, created_at, status)],
            parallel_tool_calls: request.response_parallel_tool_calls(),
            previous_response_id: None,
            prompt_cache_key: request.response_prompt_cache_key(),
            reasoning: ResponseReasoning {
                effort: None,
                summary: None,
            },
            safety_identifier: request.response_safety_identifier(),
            service_tier: request.response_service_tier().unwrap_or("default"),
            store: false,
            temperature: sampling.temperature(),
            text: ResponseTextConfiguration {
                format: ResponseTextFormat { kind: "text" },
                verbosity: "medium",
            },
            tool_choice: request.response_tool_choice(),
            tools: Vec::new(),
            top_logprobs: 0,
            top_p: sampling.top_p(),
            truncation: "disabled",
            usage: ResponsesUsage::from_generation(&generated),
            user: request
                .user
                .as_ref()
                .and_then(Value::as_str)
                .map(str::to_owned),
            metadata: request.response_metadata(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct IncompleteDetails {
    reason: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct ResponseOutputMessage {
    id: String,
    #[serde(rename = "type")]
    kind: &'static str,
    status: &'static str,
    role: &'static str,
    content: Vec<ResponseOutputText>,
}

impl ResponseOutputMessage {
    fn new(generated: &GeneratedText, created_at: u64, status: &'static str) -> Self {
        Self {
            id: response_id("msg", created_at),
            kind: "message",
            status,
            role: "assistant",
            content: vec![ResponseOutputText {
                kind: "output_text",
                text: generated.text().to_owned(),
                annotations: Vec::new(),
                logprobs: Vec::new(),
            }],
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct ResponseOutputText {
    #[serde(rename = "type")]
    kind: &'static str,
    text: String,
    annotations: Vec<Value>,
    logprobs: Vec<Value>,
}

#[derive(Clone, Debug, Serialize)]
struct ResponseReasoning {
    effort: Option<Value>,
    summary: Option<Value>,
}

#[derive(Clone, Debug, Serialize)]
struct ResponseTextConfiguration {
    format: ResponseTextFormat,
    verbosity: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct ResponseTextFormat {
    #[serde(rename = "type")]
    kind: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct ResponsesUsage {
    input_tokens: usize,
    input_tokens_details: ResponseInputTokensDetails,
    output_tokens: usize,
    output_tokens_details: ResponseOutputTokensDetails,
    total_tokens: usize,
}

impl ResponsesUsage {
    fn from_generation(generated: &GeneratedText) -> Self {
        Self {
            input_tokens: generated.prompt_tokens(),
            input_tokens_details: ResponseInputTokensDetails {
                cached_tokens: generated.cached_prompt_tokens(),
            },
            output_tokens: generated.completion_tokens(),
            output_tokens_details: ResponseOutputTokensDetails {
                reasoning_tokens: 0,
            },
            total_tokens: generated.prompt_tokens() + generated.completion_tokens(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct ResponseInputTokensDetails {
    cached_tokens: usize,
}

#[derive(Clone, Debug, Serialize)]
struct ResponseOutputTokensDetails {
    reasoning_tokens: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn string_input_and_instructions_become_chat_messages() -> Result<(), Box<dyn std::error::Error>>
    {
        let request: ResponsesRequest = serde_json::from_value(json!({
            "model": "fixture-model",
            "instructions": "Be concise.",
            "input": "Hello"
        }))?;

        let messages = request.messages()?;

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role(), ChatRole::System);
        assert_eq!(messages[0].content(), "Be concise.");
        assert_eq!(messages[1].role(), ChatRole::User);
        assert_eq!(messages[1].content(), "Hello");
        Ok(())
    }

    #[test]
    fn message_input_accepts_role_appropriate_text_parts() -> Result<(), Box<dyn std::error::Error>>
    {
        let request: ResponsesRequest = serde_json::from_value(json!({
            "model": "fixture-model",
            "input": [
                {"role":"user","content":[{"type":"input_text","text":"Hello"}]},
                {"type":"message","role":"assistant","content":[{"type":"output_text","text":"Hi"}]}
            ]
        }))?;

        let messages = request.messages()?;

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role(), ChatRole::User);
        assert_eq!(messages[1].role(), ChatRole::Assistant);
        Ok(())
    }

    #[test]
    fn input_rejects_non_text_items_and_excessive_counts() -> Result<(), Box<dyn std::error::Error>>
    {
        let image_request: ResponsesRequest = serde_json::from_value(json!({
            "model": "fixture-model",
            "input": [{"role":"user","content":[{"type":"input_image","image_url":"data:image/png;base64,AA=="}]}]
        }))?;
        let too_many = (0..=MAX_INPUT_MESSAGES)
            .map(|_| json!({"role":"user","content":"hello"}))
            .collect::<Vec<_>>();
        let oversized_request: ResponsesRequest = serde_json::from_value(json!({
            "model": "fixture-model",
            "input": too_many
        }))?;

        let image_error = match image_request.messages() {
            Ok(_) => return Err("multimodal input should fail".into()),
            Err(error) => error,
        };
        let oversized_error = match oversized_request.messages() {
            Ok(_) => return Err("oversized message input should fail".into()),
            Err(error) => error,
        };
        assert_eq!(image_error.parameter(), "input");
        assert!(oversized_error.to_string().contains("more than"));
        Ok(())
    }

    #[test]
    fn neutral_response_options_are_accepted_but_stateful_features_are_not()
    -> Result<(), Box<dyn std::error::Error>> {
        let neutral: ResponsesRequest = serde_json::from_value(json!({
            "model":"fixture-model",
            "input":"hello",
            "background":false,
            "store":false,
            "include":[],
            "tools":[],
            "tool_choice":"auto",
            "text":{"format":{"type":"text"},"verbosity":"medium"},
            "reasoning":{"effort":null,"summary":null},
            "truncation":"disabled"
        }))?;
        let unsupported: ResponsesRequest = serde_json::from_value(json!({
            "model":"fixture-model",
            "input":"hello",
            "background":true,
            "previous_response_id":"resp_remote",
            "tools":[{"type":"function","name":"lookup"}],
            "truncation":"auto"
        }))?;

        assert!(neutral.unsupported_fields().is_empty());
        assert_eq!(
            unsupported.unsupported_fields(),
            ["background", "previous_response_id", "tools", "truncation"]
        );
        Ok(())
    }

    #[test]
    fn response_shape_reports_incomplete_length_and_usage() -> Result<(), Box<dyn std::error::Error>>
    {
        let request: ResponsesRequest = serde_json::from_value(json!({
            "model":"fixture-model",
            "input":"hello",
            "instructions":"Answer briefly.",
            "max_output_tokens":1,
            "metadata":{"trace":"local"},
            "prompt_cache_key":"tenant-a:hello",
            "safety_identifier":"hashed-local-user",
            "tool_choice":"none"
        }))?;
        let generated = GeneratedText::with_finish_reason(
            "winner".to_owned(),
            4,
            1,
            vec!["winner".to_owned()],
            GenerationFinishReason::Length,
        )
        .with_cached_prompt_tokens(3)?;

        let response = serde_json::to_value(ResponsesResponse::from_generation(
            &request,
            "fixture-model".to_owned(),
            &SamplingConfig::default(),
            generated,
        ))?;

        assert_eq!(response["object"], "response");
        assert_eq!(response["status"], "incomplete");
        assert_eq!(response["max_output_tokens"], 1);
        assert_eq!(
            response["incomplete_details"]["reason"],
            "max_output_tokens"
        );
        assert_eq!(response["output"][0]["content"][0]["type"], "output_text");
        assert_eq!(response["output"][0]["content"][0]["text"], "winner");
        assert_eq!(response["usage"]["input_tokens"], 4);
        assert_eq!(
            response["usage"]["input_tokens_details"]["cached_tokens"],
            3
        );
        assert_eq!(response["usage"]["total_tokens"], 5);
        assert_eq!(response["metadata"]["trace"], "local");
        assert_eq!(response["prompt_cache_key"], "tenant-a:hello");
        assert_eq!(response["safety_identifier"], "hashed-local-user");
        assert_eq!(response["tool_choice"], "none");
        Ok(())
    }

    #[test]
    fn response_preserves_an_omitted_max_output_token_limit_as_null()
    -> Result<(), Box<dyn std::error::Error>> {
        let request: ResponsesRequest = serde_json::from_value(json!({
            "model":"fixture-model",
            "input":"hello"
        }))?;
        let generated = GeneratedText::with_finish_reason(
            "winner".to_owned(),
            1,
            1,
            vec!["winner".to_owned()],
            GenerationFinishReason::Stop,
        );

        let response = serde_json::to_value(ResponsesResponse::from_generation(
            &request,
            "fixture-model".to_owned(),
            &SamplingConfig::default(),
            generated,
        ))?;

        assert!(response["max_output_tokens"].is_null());
        Ok(())
    }
}
