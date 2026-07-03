use crate::runtime::{GeneratedText, PromptCacheTrace};
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
            prompt_tokens_details: PromptTokensDetails::new(
                generated.cached_prompt_tokens(),
                generated.prompt_cache_trace(),
            ),
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
            prompt_tokens_details: PromptTokensDetails::new(cached_tokens, None),
            completion_tokens_details: CompletionTokensDetails::zero(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct PromptTokensDetails {
    cached_tokens: usize,
    audio_tokens: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    ferrite_cache: Option<FerritePromptCacheTrace>,
}

impl PromptTokensDetails {
    fn new(cached_tokens: usize, trace: Option<&PromptCacheTrace>) -> Self {
        Self {
            cached_tokens,
            audio_tokens: 0,
            ferrite_cache: trace.map(FerritePromptCacheTrace::from),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct FerritePromptCacheTrace {
    enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    namespace: Option<String>,
    prompt_token_count: usize,
    prompt_token_hash: String,
    lookup: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    selected_entry_token_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    selected_entry_token_hash: Option<String>,
    shared_prefix_tokens: usize,
}

impl From<&PromptCacheTrace> for FerritePromptCacheTrace {
    fn from(trace: &PromptCacheTrace) -> Self {
        Self {
            enabled: trace.enabled(),
            namespace: trace.namespace().map(str::to_owned),
            prompt_token_count: trace.prompt_token_count(),
            prompt_token_hash: format_hash(trace.prompt_token_hash()),
            lookup: trace.lookup().as_str(),
            selected_entry_token_count: trace.selected_entry_token_count(),
            selected_entry_token_hash: trace.selected_entry_token_hash().map(format_hash),
            shared_prefix_tokens: trace.shared_prefix_tokens(),
        }
    }
}

fn format_hash(hash: u64) -> String {
    format!("fnv64:{hash:016x}")
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
    fn usage_omits_ferrite_cache_trace_by_default() -> Result<(), Box<dyn std::error::Error>> {
        let generated = GeneratedText::new("winner".to_owned(), 5, 2, vec!["winner".to_owned()])
            .with_cached_prompt_tokens(3)?;

        let usage = serde_json::to_value(Usage::from_generation(&generated))?;

        assert!(usage["prompt_tokens_details"]
            .get("ferrite_cache")
            .is_none());
        Ok(())
    }

    #[test]
    fn usage_reports_ferrite_cache_trace_when_requested() -> Result<(), Box<dyn std::error::Error>>
    {
        let trace = crate::runtime::PromptCacheTrace::new(
            true,
            Some("tenant-a:thread-1".to_owned()),
            5,
            0x1234,
            crate::runtime::PromptCacheLookup::SharedPrefixHit,
        )
        .with_selected_entry(3, 0x4567)
        .with_shared_prefix_tokens(3);
        let generated = GeneratedText::new("winner".to_owned(), 5, 2, vec!["winner".to_owned()])
            .with_cached_prompt_tokens(3)?
            .with_prompt_cache_trace(trace)?;

        let usage = serde_json::to_value(Usage::from_generation(&generated))?;
        let trace = &usage["prompt_tokens_details"]["ferrite_cache"];

        assert_eq!(trace["enabled"], true);
        assert_eq!(trace["namespace"], "tenant-a:thread-1");
        assert_eq!(trace["prompt_token_count"], 5);
        assert_eq!(trace["prompt_token_hash"], "fnv64:0000000000001234");
        assert_eq!(trace["lookup"], "shared_prefix_hit");
        assert_eq!(trace["selected_entry_token_count"], 3);
        assert_eq!(trace["selected_entry_token_hash"], "fnv64:0000000000004567");
        assert_eq!(trace["shared_prefix_tokens"], 3);
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
