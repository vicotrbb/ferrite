use crate::runtime::GeneratedText;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(super) struct Usage {
    prompt_tokens: usize,
    completion_tokens: usize,
    total_tokens: usize,
}

impl Usage {
    pub(super) fn from_generation(generated: &GeneratedText) -> Self {
        Self {
            prompt_tokens: generated.prompt_tokens(),
            completion_tokens: generated.completion_tokens(),
            total_tokens: generated.prompt_tokens() + generated.completion_tokens(),
        }
    }

    pub(super) fn from_generations(generated: &[GeneratedText]) -> Self {
        let prompt_tokens = generated.iter().map(GeneratedText::prompt_tokens).sum();
        let completion_tokens = generated.iter().map(GeneratedText::completion_tokens).sum();
        Self {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
        }
    }
}
