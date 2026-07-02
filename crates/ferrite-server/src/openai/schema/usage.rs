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
            prompt_tokens_details: PromptTokensDetails::new(generated.cached_prompt_tokens()),
            completion_tokens_details: CompletionTokensDetails::zero(),
        }
    }

    pub(super) fn from_generations(generated: &[GeneratedText]) -> Self {
        let prompt_tokens = generated.iter().map(GeneratedText::prompt_tokens).sum();
        let completion_tokens = generated.iter().map(GeneratedText::completion_tokens).sum();
        let cached_tokens = generated
            .iter()
            .map(GeneratedText::cached_prompt_tokens)
            .sum();
        Self {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
            prompt_tokens_details: PromptTokensDetails::new(cached_tokens),
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
    fn new(cached_tokens: usize) -> Self {
        Self {
            cached_tokens,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::GeneratedText;
    use serde_json::json;

    #[test]
    fn usage_reports_cached_prompt_tokens_from_generation() -> Result<(), Box<dyn std::error::Error>>
    {
        let generated = GeneratedText::new("winner".to_owned(), 5, 2, vec!["winner".to_owned()])
            .with_cached_prompt_tokens(3)?;

        let usage = serde_json::to_value(Usage::from_generation(&generated))?;

        assert_eq!(usage["prompt_tokens"], 5);
        assert_eq!(usage["completion_tokens"], 2);
        assert_eq!(usage["total_tokens"], 7);
        assert_eq!(usage["prompt_tokens_details"]["cached_tokens"], 3);
        assert_eq!(usage["prompt_tokens_details"]["audio_tokens"], 0);
        Ok(())
    }

    #[test]
    fn usage_sums_cached_prompt_tokens_for_multiple_generations(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let first = GeneratedText::new("first".to_owned(), 5, 2, vec!["first".to_owned()])
            .with_cached_prompt_tokens(3)?;
        let second = GeneratedText::new("second".to_owned(), 7, 4, vec!["second".to_owned()])
            .with_cached_prompt_tokens(2)?;

        let usage = serde_json::to_value(Usage::from_generations(&[first, second]))?;

        assert_eq!(
            usage["prompt_tokens_details"],
            json!({"cached_tokens":5,"audio_tokens":0})
        );
        assert_eq!(usage["prompt_tokens"], 12);
        assert_eq!(usage["completion_tokens"], 6);
        assert_eq!(usage["total_tokens"], 18);
        Ok(())
    }
}
