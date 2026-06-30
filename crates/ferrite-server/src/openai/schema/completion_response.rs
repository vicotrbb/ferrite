use super::{id::response_id, unix_timestamp, usage::Usage};
use crate::runtime::{GeneratedText, GenerationFinishReason};
use serde::Serialize;
use serde_json::Value;

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
