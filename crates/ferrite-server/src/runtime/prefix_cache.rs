use super::{
    GenerationCacheOptions, InferenceEngine, PromptCacheLookup, PromptCacheTrace, RuntimeError,
};
use ferrite_inference::prefix_cache::{
    PrefixCacheEntry, PrefixCacheFingerprints, PrefixCacheKey, PrefixCacheMetadataStore,
    TokenPrefixIdentity,
};
use ferrite_inference::scalar::{KvBackend, NextToken, ScalarLlamaSessionSnapshot};
use std::collections::HashMap;
use std::sync::Arc;

const DEFAULT_MAX_PREFIX_CACHE_ENTRIES: usize = 8;
const DEFAULT_MAX_PREFIX_CACHE_BYTES: u128 = 64 * 1024 * 1024;

#[derive(Debug)]
pub(super) struct RuntimePrefixCache {
    metadata: PrefixCacheMetadataStore,
    values: HashMap<PrefixCacheKey, RuntimePrefixCacheValue>,
    max_entries: usize,
    max_bytes: u128,
    next_tick: u64,
}

impl Default for RuntimePrefixCache {
    fn default() -> Self {
        Self::new(
            DEFAULT_MAX_PREFIX_CACHE_ENTRIES,
            DEFAULT_MAX_PREFIX_CACHE_BYTES,
        )
    }
}

impl RuntimePrefixCache {
    pub(super) fn new(max_entries: usize, max_bytes: u128) -> Self {
        Self {
            metadata: PrefixCacheMetadataStore::new(max_entries, max_bytes),
            values: HashMap::new(),
            max_entries,
            max_bytes,
            next_tick: 0,
        }
    }

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
                snapshot: Arc::new(snapshot),
                next_token: None,
                next_token_id: None,
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
                    snapshot: Arc::new(snapshot),
                    next_token_id: Some(next_token.token_id),
                    next_token: Some(next_token),
                },
            );
        }
    }

    pub(super) fn insert_greedy(
        &mut self,
        key: PrefixCacheKey,
        snapshot: ScalarLlamaSessionSnapshot,
        next_token_id: usize,
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
                    snapshot: Arc::new(snapshot),
                    next_token: None,
                    next_token_id: Some(next_token_id),
                },
            );
        }
    }

    fn advance_tick(&mut self) -> u64 {
        self.next_tick = self.next_tick.saturating_add(1);
        self.next_tick
    }

    pub(super) fn stats(&self) -> PrefixCacheStats {
        PrefixCacheStats {
            entries: self.values.len(),
            estimated_kv_bytes: self.metadata.estimated_kv_bytes(),
            max_entries: self.max_entries,
            max_bytes: self.max_bytes,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrefixCacheStats {
    entries: usize,
    estimated_kv_bytes: u128,
    max_entries: usize,
    max_bytes: u128,
}

impl PrefixCacheStats {
    pub fn entries(self) -> usize {
        self.entries
    }

    pub fn estimated_kv_bytes(self) -> u128 {
        self.estimated_kv_bytes
    }

    pub fn max_entries(self) -> usize {
        self.max_entries
    }

    pub fn max_bytes(self) -> u128 {
        self.max_bytes
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct RuntimePrefixCacheValue {
    snapshot: Arc<ScalarLlamaSessionSnapshot>,
    next_token: Option<NextToken>,
    next_token_id: Option<usize>,
}

impl RuntimePrefixCacheValue {
    pub(super) fn snapshot(&self) -> &ScalarLlamaSessionSnapshot {
        self.snapshot.as_ref()
    }

    pub(super) fn next_token(&self) -> Option<&NextToken> {
        self.next_token.as_ref()
    }

    pub(super) fn next_token_id(&self) -> Option<usize> {
        self.next_token_id
    }

    #[cfg(test)]
    pub(super) fn snapshot_owner_count(&self) -> usize {
        Arc::strong_count(&self.snapshot)
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
    pub fn with_prefix_cache_limits(
        mut self,
        max_entries: usize,
        max_bytes: u128,
    ) -> Result<Self, RuntimeError> {
        if max_entries == 0 {
            return Err(RuntimeError::new(
                "prefix cache max entries must be greater than zero",
            ));
        }
        if max_bytes == 0 {
            return Err(RuntimeError::new(
                "prefix cache max bytes must be greater than zero",
            ));
        }
        self.prefix_cache = std::sync::Mutex::new(RuntimePrefixCache::new(max_entries, max_bytes));
        Ok(self)
    }

    pub fn prefix_cache_stats(&self) -> Result<PrefixCacheStats, RuntimeError> {
        self.prefix_cache
            .lock()
            .map_err(|_error| RuntimeError::new("runtime prefix cache lock is poisoned"))
            .map(|cache| cache.stats())
    }

    pub(super) fn prefix_cache_lookup(
        &self,
        key: &PrefixCacheKey,
    ) -> Result<RuntimePrefixCacheLookup, RuntimeError> {
        self.prefix_cache
            .lock()
            .map_err(|_error| RuntimeError::new("runtime prefix cache lock is poisoned"))
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
            .map_err(|_error| RuntimeError::new("runtime prefix cache lock is poisoned"))
            .map(|mut cache| cache.insert(key, snapshot, next_token))
    }

    pub(super) fn store_prefix_cache_greedy_value(
        &self,
        key: PrefixCacheKey,
        snapshot: ScalarLlamaSessionSnapshot,
        next_token_id: usize,
    ) -> Result<(), RuntimeError> {
        self.prefix_cache
            .lock()
            .map_err(|_error| RuntimeError::new("runtime prefix cache lock is poisoned"))
            .map(|mut cache| cache.insert_greedy(key, snapshot, next_token_id))
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
                self.chat_template_fingerprint.clone(),
                self.execution_fingerprint(),
                "text-generation-v1",
            ),
            TokenPrefixIdentity::from_tokens(prompt_token_ids.iter().copied()),
        );
        if let Some(namespace) = cache_options.namespace() {
            key = key.with_namespace(namespace);
        }
        key
    }

    fn execution_fingerprint(&self) -> String {
        let policy = self.execution_options.q8_k_activation_matvec_policy();
        let kv = match self.execution_options.kv_backend() {
            KvBackend::Vec => "kv=vec".to_owned(),
            KvBackend::Locus {
                tokens_per_block,
                max_tokens,
            } => format!("kv=locus:block={tokens_per_block}:max={max_tokens}"),
        };
        format!(
            "scalar:{}:kernels={}:{kv}",
            policy.as_str(),
            self.execution_options.kernel_provider().as_str()
        )
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
