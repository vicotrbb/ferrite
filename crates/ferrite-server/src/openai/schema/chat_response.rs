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
        let created = unix_timestamp();
        Self {
            id: response_id("chatcmpl", created),
            object: "chat.completion",
            created,
            model,
            system_fingerprint: None,
            choices: vec![ChatCompletionChoice::new(&generated)],
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
    fn new(generated: &GeneratedText) -> Self {
        Self {
            index: 0,
            message: ChatCompletionMessage::assistant(generated.text().to_owned()),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_completion_response_ids_are_unique_within_the_same_second(
    ) -> Result<(), Box<dyn std::error::Error>> {
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

    fn generated() -> GeneratedText {
        GeneratedText::new("winner".to_owned(), 1, 1, vec!["winner".to_owned()])
    }
}
