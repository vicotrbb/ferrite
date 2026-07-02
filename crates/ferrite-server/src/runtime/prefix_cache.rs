use super::{GenerationCacheOptions, InferenceEngine, RuntimeError};
use ferrite_inference::prefix_cache::{
    PrefixCacheFingerprints, PrefixCacheKey, TokenPrefixIdentity,
};

impl InferenceEngine {
    pub fn prefix_cache_key_for_prompt(
        &self,
        prompt: &str,
        cache_options: &GenerationCacheOptions,
    ) -> Result<PrefixCacheKey, RuntimeError> {
        let prompt_token_ids = self
            .tokenizer
            .encode(prompt)
            .map_err(|error| RuntimeError::new(format!("failed to tokenize prompt: {error}")))?;
        if prompt_token_ids.is_empty() {
            return Err(RuntimeError::new("prompt must contain at least one token"));
        }
        Ok(self.prefix_cache_key_for_tokens(&prompt_token_ids, cache_options))
    }

    pub(super) fn prefix_cache_key_for_tokens(
        &self,
        prompt_token_ids: &[usize],
        cache_options: &GenerationCacheOptions,
    ) -> PrefixCacheKey {
        let mut key = PrefixCacheKey::new(
            PrefixCacheFingerprints::new(
                self.model_fingerprint.clone(),
                self.tokenizer_fingerprint.clone(),
                "runtime-rendered-prompt-v1",
                "scalar-default",
                "text-generation-v1",
            ),
            TokenPrefixIdentity::from_tokens(prompt_token_ids.iter().copied()),
        );
        if let Some(namespace) = cache_options.namespace() {
            key = key.with_namespace(namespace);
        }
        key
    }
}

const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

pub(super) fn fnv64_bytes(bytes: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}
