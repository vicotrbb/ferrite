use super::tool_options::{ParsedAssistantOutput, ParsedToolCall};
use super::{id::response_id, unix_timestamp, usage::Usage};
use crate::runtime::{GeneratedText, GenerationFinishReason};
use serde::Serialize;
use serde_json::Value;

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
        let output = ParsedAssistantOutput {
            content: Some(generated.text().to_owned()),
            tool_calls: Vec::new(),
        };
        Self::from_generation_with_tool_output(model, generated, service_tier, output)
    }

    pub(crate) fn from_generation_with_tool_output(
        model: String,
        generated: GeneratedText,
        service_tier: Option<&'static str>,
        output: ParsedAssistantOutput,
    ) -> Self {
        let created = unix_timestamp();
        Self {
            id: response_id("chatcmpl", created),
            object: "chat.completion",
            created,
            model,
            system_fingerprint: None,
            choices: vec![ChatCompletionChoice::new(&generated, output, created)],
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
    fn new(generated: &GeneratedText, output: ParsedAssistantOutput, created: u64) -> Self {
        let has_tool_calls = !output.tool_calls.is_empty();
        Self {
            index: 0,
            message: ChatCompletionMessage::assistant(output, created),
            logprobs: None,
            finish_reason: if has_tool_calls {
                "tool_calls"
            } else {
                finish_reason(generated.finish_reason())
            },
        }
    }
}

fn finish_reason(reason: GenerationFinishReason) -> &'static str {
    match reason {
        GenerationFinishReason::Stop => "stop",
        GenerationFinishReason::Length => "length",
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct ChatCompletionMessage {
    role: &'static str,
    content: Option<String>,
    refusal: Option<String>,
    annotations: Vec<Value>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tool_calls: Vec<ResponseToolCall>,
}

impl ChatCompletionMessage {
    fn assistant(output: ParsedAssistantOutput, created: u64) -> Self {
        Self {
            role: "assistant",
            content: output.content,
            refusal: None,
            annotations: Vec::new(),
            tool_calls: output
                .tool_calls
                .into_iter()
                .map(|tool_call| ResponseToolCall::new(tool_call, created))
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct ResponseToolCall {
    id: String,
    #[serde(rename = "type")]
    kind: &'static str,
    function: ResponseToolCallFunction,
}

impl ResponseToolCall {
    fn new(tool_call: ParsedToolCall, created: u64) -> Self {
        Self {
            id: response_id("call", created),
            kind: "function",
            function: ResponseToolCallFunction {
                name: tool_call.name,
                arguments: tool_call.arguments,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct ResponseToolCallFunction {
    name: String,
    arguments: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_completion_response_ids_are_unique_within_the_same_second()
    -> Result<(), Box<dyn std::error::Error>> {
        for _ in 0..1_000 {
            let first = ChatCompletionResponse::from_generation(
                "fixture-model".to_owned(),
                generated(),
                None,
            );
            let second = ChatCompletionResponse::from_generation(
                "fixture-model".to_owned(),
                generated(),
                None,
            );

            if first.created == second.created {
                assert_ne!(first.id, second.id);
                return Ok(());
            }
        }

        Err("expected to create two chat completion responses in the same second".into())
    }

    #[test]
    fn tool_generation_uses_openai_tool_call_shape() -> Result<(), Box<dyn std::error::Error>> {
        let response = ChatCompletionResponse::from_generation_with_tool_output(
            "fixture-model".to_owned(),
            generated(),
            None,
            ParsedAssistantOutput {
                content: None,
                tool_calls: vec![ParsedToolCall {
                    name: "lookup".to_owned(),
                    arguments: r#"{"query":"rust"}"#.to_owned(),
                }],
            },
        );
        let value = serde_json::to_value(response)?;

        assert_eq!(value["choices"][0]["finish_reason"], "tool_calls");
        assert!(value["choices"][0]["message"]["content"].is_null());
        assert_eq!(
            value["choices"][0]["message"]["tool_calls"][0]["function"]["name"],
            "lookup"
        );
        Ok(())
    }

    fn generated() -> GeneratedText {
        GeneratedText::new("winner".to_owned(), 1, 1, vec!["winner".to_owned()])
    }
}
