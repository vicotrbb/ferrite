use super::{
    GenerationCacheOptions, InferenceEngine, PromptCacheLookup, PromptCacheTrace, RuntimeError,
};
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
    pub(super) fn lookup_longest_prefix(
        &mut self,
        key: &PrefixCacheKey,
    ) -> RuntimePrefixCacheLookup {
        let used_at_tick = self.advance_tick();
        if let Some(entry) = self.metadata.record_longest_prefix_hit(key, used_at_tick) {
            let selected_entry_token_count = entry.key().prefix_token_count();
            let selected_entry_token_hash = entry.key().prefix_token_hash();
            let lookup = if selected_entry_token_count == key.prefix_token_count() {
                PromptCacheLookup::ExactHit
            } else {
                PromptCacheLookup::PrefixHit
            };
            let Some(value) = self.values.get(entry.key()).cloned() else {
                return RuntimePrefixCacheLookup::miss();
            };
            return RuntimePrefixCacheLookup {
                value: Some(value),
                lookup,
                selected_entry_token_count: Some(selected_entry_token_count),
                selected_entry_token_hash: Some(selected_entry_token_hash),
                shared_prefix_tokens: selected_entry_token_count,
            };
        }
        let Some(hit) = self
            .metadata
            .record_longest_shared_prefix_hit(key, used_at_tick)
        else {
            return RuntimePrefixCacheLookup::miss();
        };
        let selected_entry_token_count = hit.entry().key().prefix_token_count();
        let selected_entry_token_hash = hit.entry().key().prefix_token_hash();
        let shared_prefix_tokens = hit.shared_prefix_token_count();
        let Some(snapshot) = self.values.get(hit.entry().key()).and_then(|value| {
            value
                .snapshot
                .truncate_to_cached_token_count(shared_prefix_tokens)
                .ok()
        }) else {
            return RuntimePrefixCacheLookup::miss();
        };
        RuntimePrefixCacheLookup {
            value: Some(RuntimePrefixCacheValue {
                snapshot,
                next_token: None,
            }),
            lookup: PromptCacheLookup::SharedPrefixHit,
            selected_entry_token_count: Some(selected_entry_token_count),
            selected_entry_token_hash: Some(selected_entry_token_hash),
            shared_prefix_tokens,
        }
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
                    next_token: Some(next_token),
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
    next_token: Option<NextToken>,
}

impl RuntimePrefixCacheValue {
    pub(super) fn snapshot(&self) -> &ScalarLlamaSessionSnapshot {
        &self.snapshot
    }

    pub(super) fn next_token(&self) -> Option<&NextToken> {
        self.next_token.as_ref()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct RuntimePrefixCacheLookup {
    value: Option<RuntimePrefixCacheValue>,
    lookup: PromptCacheLookup,
    selected_entry_token_count: Option<usize>,
    selected_entry_token_hash: Option<u64>,
    shared_prefix_tokens: usize,
}

impl RuntimePrefixCacheLookup {
    fn miss() -> Self {
        Self {
            value: None,
            lookup: PromptCacheLookup::Miss,
            selected_entry_token_count: None,
            selected_entry_token_hash: None,
            shared_prefix_tokens: 0,
        }
    }

    pub(super) fn into_value(self) -> Option<RuntimePrefixCacheValue> {
        self.value
    }

    pub(super) fn to_trace(&self, key: &PrefixCacheKey, enabled: bool) -> PromptCacheTrace {
        let mut trace = PromptCacheTrace::new(
            enabled,
            key.namespace().map(str::to_owned),
            key.prefix_token_count(),
            key.prefix_token_hash(),
            self.lookup,
        )
        .with_shared_prefix_tokens(self.shared_prefix_tokens);
        if let (Some(token_count), Some(token_hash)) = (
            self.selected_entry_token_count,
            self.selected_entry_token_hash,
        ) {
            trace = trace.with_selected_entry(token_count, token_hash);
        }
        trace
    }
}

impl InferenceEngine {
    pub(super) fn prefix_cache_lookup(
        &self,
        key: &PrefixCacheKey,
    ) -> Result<RuntimePrefixCacheLookup, RuntimeError> {
        self.prefix_cache
            .lock()
            .map_err(|_| RuntimeError::new("runtime prefix cache lock is poisoned"))
            .map(|mut cache| cache.lookup_longest_prefix(key))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_misses_when_exact_metadata_entry_has_no_value() {
        let mut cache = RuntimePrefixCache::default();
        let key = test_key(&[1, 2, 3]);
        cache
            .metadata
            .insert(PrefixCacheEntry::new(key.clone(), 1024, 1));

        let lookup = cache.lookup_longest_prefix(&key);

        assert_eq!(lookup.lookup, PromptCacheLookup::Miss);
        assert!(lookup.value.is_none());
        assert_eq!(lookup.shared_prefix_tokens, 0);
    }

    #[test]
    fn lookup_misses_when_shared_metadata_entry_has_no_value() {
        let mut cache = RuntimePrefixCache::default();
        let cached_key = test_key(&[1, 2, 3]);
        let requested_key = test_key(&[1, 4, 5]);
        cache
            .metadata
            .insert(PrefixCacheEntry::new(cached_key, 1024, 1));

        let lookup = cache.lookup_longest_prefix(&requested_key);

        assert_eq!(lookup.lookup, PromptCacheLookup::Miss);
        assert!(lookup.value.is_none());
        assert_eq!(lookup.shared_prefix_tokens, 0);
    }

    fn test_key(tokens: &[usize]) -> PrefixCacheKey {
        PrefixCacheKey::new(
            PrefixCacheFingerprints::new(
                "model",
                "tokenizer",
                "template",
                "execution",
                "request-shape",
            ),
            TokenPrefixIdentity::from_tokens(tokens.iter().copied()),
        )
        .with_namespace("tenant-a:thread-1")
    }
}
