use super::{GenerationCacheOptions, InferenceEngine, RuntimeError};
use ferrite_inference::prefix_cache::{
    PrefixCacheEntry, PrefixCacheFingerprints, PrefixCacheKey, PrefixCacheMetadataStore,
    TokenPrefixIdentity,
};
use ferrite_inference::scalar::{NextToken, ScalarLlamaSessionSnapshot};
use std::collections::HashMap;

const DEFAULT_MAX_PREFIX_CACHE_ENTRIES: usize = 8;
const DEFAULT_MAX_PREFIX_CACHE_BYTES: u128 = 64 * 1024 * 1024;

#[derive(Debug)]
pub(super) struct RuntimePrefixCache {
    metadata: PrefixCacheMetadataStore,
    values: HashMap<PrefixCacheKey, RuntimePrefixCacheValue>,
    next_tick: u64,
}

impl Default for RuntimePrefixCache {
    fn default() -> Self {
        Self {
            metadata: PrefixCacheMetadataStore::new(
                DEFAULT_MAX_PREFIX_CACHE_ENTRIES,
                DEFAULT_MAX_PREFIX_CACHE_BYTES,
            ),
            values: HashMap::new(),
            next_tick: 0,
        }
    }
}

impl RuntimePrefixCache {
    pub(super) fn get_longest_prefix(
        &mut self,
        key: &PrefixCacheKey,
    ) -> Option<RuntimePrefixCacheValue> {
        let used_at_tick = self.advance_tick();
        let entry = self.metadata.record_longest_prefix_hit(key, used_at_tick)?;
        self.values.get(entry.key()).cloned()
    }

    pub(super) fn insert(
        &mut self,
        key: PrefixCacheKey,
        snapshot: ScalarLlamaSessionSnapshot,
        next_token: NextToken,
    ) {
        let created_at_tick = self.advance_tick();
        let metadata =
            PrefixCacheEntry::new(key.clone(), snapshot.kv_cache_bytes(), created_at_tick);
        for evicted in self.metadata.insert(metadata) {
            self.values.remove(evicted.key());
        }
        if self.metadata.get(&key).is_some() {
            self.values.insert(
                key,
                RuntimePrefixCacheValue {
                    snapshot,
                    next_token,
                },
            );
        }
    }

    fn advance_tick(&mut self) -> u64 {
        self.next_tick = self.next_tick.saturating_add(1);
        self.next_tick
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct RuntimePrefixCacheValue {
    snapshot: ScalarLlamaSessionSnapshot,
    next_token: NextToken,
}

impl RuntimePrefixCacheValue {
    pub(super) fn snapshot(&self) -> &ScalarLlamaSessionSnapshot {
        &self.snapshot
    }

    pub(super) fn next_token(&self) -> &NextToken {
        &self.next_token
    }
}

impl InferenceEngine {
    pub(super) fn prefix_cache_hit(
        &self,
        key: &PrefixCacheKey,
    ) -> Result<Option<RuntimePrefixCacheValue>, RuntimeError> {
        self.prefix_cache
            .lock()
            .map_err(|_| RuntimeError::new("runtime prefix cache lock is poisoned"))
            .map(|mut cache| cache.get_longest_prefix(key))
    }

    pub(super) fn store_prefix_cache_value(
        &self,
        key: PrefixCacheKey,
        snapshot: ScalarLlamaSessionSnapshot,
        next_token: NextToken,
    ) -> Result<(), RuntimeError> {
        self.prefix_cache
            .lock()
            .map_err(|_| RuntimeError::new("runtime prefix cache lock is poisoned"))
            .map(|mut cache| cache.insert(key, snapshot, next_token))
    }

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
