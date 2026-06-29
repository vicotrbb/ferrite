use crate::runtime::GeneratedText;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(super) struct Usage {
    prompt_tokens: usize,
    completion_tokens: usize,
    total_tokens: usize,
    prompt_tokens_details: PromptTokensDetails,
    completion_tokens_details: CompletionTokensDetails,
}

impl Usage {
    pub(super) fn from_generation(generated: &GeneratedText) -> Self {
        Self {
            prompt_tokens: generated.prompt_tokens(),
            completion_tokens: generated.completion_tokens(),
            total_tokens: generated.prompt_tokens() + generated.completion_tokens(),
            prompt_tokens_details: PromptTokensDetails::zero(),
            completion_tokens_details: CompletionTokensDetails::zero(),
        }
    }

    pub(super) fn from_generations(generated: &[GeneratedText]) -> Self {
        let prompt_tokens = generated.iter().map(GeneratedText::prompt_tokens).sum();
        let completion_tokens = generated.iter().map(GeneratedText::completion_tokens).sum();
        Self {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
            prompt_tokens_details: PromptTokensDetails::zero(),
            completion_tokens_details: CompletionTokensDetails::zero(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct PromptTokensDetails {
    cached_tokens: usize,
    audio_tokens: usize,
}

impl PromptTokensDetails {
    fn zero() -> Self {
        Self {
            cached_tokens: 0,
            audio_tokens: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct CompletionTokensDetails {
    reasoning_tokens: usize,
    audio_tokens: usize,
    accepted_prediction_tokens: usize,
    rejected_prediction_tokens: usize,
}

impl CompletionTokensDetails {
    fn zero() -> Self {
        Self {
            reasoning_tokens: 0,
            audio_tokens: 0,
            accepted_prediction_tokens: 0,
            rejected_prediction_tokens: 0,
        }
    }
}
