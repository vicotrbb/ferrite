mod store;

pub use store::PrefixCacheMetadataStore;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
/// An owned token sequence and its stable in-process FNV-1a identity hash.
///
/// Equality includes the complete token sequence, so the hash is never used as
/// proof that two prefixes are equal.
pub struct TokenPrefixIdentity {
    tokens: Vec<usize>,
    token_hash: u64,
}

impl TokenPrefixIdentity {
    /// Builds an identity from tokens in iteration order.
    pub fn from_tokens(tokens: impl IntoIterator<Item = usize>) -> Self {
        let tokens = tokens.into_iter().collect::<Vec<_>>();
        let mut token_hash = FNV_OFFSET_BASIS;
        for token in &tokens {
            token_hash = mix_usize(token_hash, *token);
        }
        token_hash = mix_usize(token_hash, tokens.len());
        Self { tokens, token_hash }
    }

    /// Returns the number of tokens in the prefix.
    pub fn token_count(&self) -> usize {
        self.tokens.len()
    }

    /// Returns the exact prefix tokens.
    pub fn tokens(&self) -> &[usize] {
        &self.tokens
    }

    /// Returns the precomputed in-process identity hash.
    ///
    /// Callers must compare the complete identity before reusing cached state.
    pub fn token_hash(&self) -> u64 {
        self.token_hash
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
/// Compatibility dimensions that must match before cached KV state is reused.
pub struct PrefixCacheFingerprints {
    model_fingerprint: String,
    tokenizer_fingerprint: String,
    template_fingerprint: String,
    execution_fingerprint: String,
    request_shape_fingerprint: String,
}

impl PrefixCacheFingerprints {
    /// Creates a complete set of prefix-cache compatibility fingerprints.
    pub fn new(
        model_fingerprint: impl Into<String>,
        tokenizer_fingerprint: impl Into<String>,
        template_fingerprint: impl Into<String>,
        execution_fingerprint: impl Into<String>,
        request_shape_fingerprint: impl Into<String>,
    ) -> Self {
        Self {
            model_fingerprint: model_fingerprint.into(),
            tokenizer_fingerprint: tokenizer_fingerprint.into(),
            template_fingerprint: template_fingerprint.into(),
            execution_fingerprint: execution_fingerprint.into(),
            request_shape_fingerprint: request_shape_fingerprint.into(),
        }
    }

    /// Returns the model artifact fingerprint.
    pub fn model(&self) -> &str {
        &self.model_fingerprint
    }

    /// Returns the tokenizer configuration fingerprint.
    pub fn tokenizer(&self) -> &str {
        &self.tokenizer_fingerprint
    }

    /// Returns the prompt-template fingerprint.
    pub fn template(&self) -> &str {
        &self.template_fingerprint
    }

    /// Returns the inference execution-policy fingerprint.
    pub fn execution(&self) -> &str {
        &self.execution_fingerprint
    }

    /// Returns the request-shape fingerprint.
    pub fn request_shape(&self) -> &str {
        &self.request_shape_fingerprint
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
/// A fully qualified prefix-cache lookup key.
pub struct PrefixCacheKey {
    fingerprints: PrefixCacheFingerprints,
    namespace: Option<String>,
    prefix: TokenPrefixIdentity,
}

impl PrefixCacheKey {
    /// Creates a key without an application-provided namespace.
    pub fn new(fingerprints: PrefixCacheFingerprints, prefix: TokenPrefixIdentity) -> Self {
        Self {
            fingerprints,
            namespace: None,
            prefix,
        }
    }

    /// Adds a caller-defined cache namespace to the key.
    #[must_use]
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    /// Returns the optional caller-defined namespace.
    pub fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }

    /// Returns the number of prefix tokens.
    pub fn prefix_token_count(&self) -> usize {
        self.prefix.token_count()
    }

    /// Returns the exact prefix token sequence.
    pub fn prefix_tokens(&self) -> &[usize] {
        self.prefix.tokens()
    }

    /// Returns the prefix's precomputed identity hash.
    pub fn prefix_token_hash(&self) -> u64 {
        self.prefix.token_hash()
    }

    /// Returns the compatibility fingerprints.
    pub fn fingerprints(&self) -> &PrefixCacheFingerprints {
        &self.fingerprints
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// Metadata describing one cached prefix and its budget accounting.
pub struct PrefixCacheEntry {
    key: PrefixCacheKey,
    matched_prefix_token_count: usize,
    estimated_kv_bytes: u128,
    created_at_tick: u64,
    last_used_at_tick: u64,
}

impl PrefixCacheEntry {
    /// Creates an entry and marks its full key prefix as the matched prefix.
    pub fn new(key: PrefixCacheKey, estimated_kv_bytes: u128, created_at_tick: u64) -> Self {
        Self {
            matched_prefix_token_count: key.prefix_token_count(),
            key,
            estimated_kv_bytes,
            created_at_tick,
            last_used_at_tick: created_at_tick,
        }
    }

    /// Returns the lookup key.
    pub fn key(&self) -> &PrefixCacheKey {
        &self.key
    }

    /// Returns the number of cached prefix tokens represented by the entry.
    pub fn matched_prefix_token_count(&self) -> usize {
        self.matched_prefix_token_count
    }

    /// Returns the estimated KV storage charged to the cache budget.
    pub fn estimated_kv_bytes(&self) -> u128 {
        self.estimated_kv_bytes
    }

    /// Returns the logical tick at which the entry was created.
    pub fn created_at_tick(&self) -> u64 {
        self.created_at_tick
    }

    /// Returns the logical tick at which the entry was last used.
    pub fn last_used_at_tick(&self) -> u64 {
        self.last_used_at_tick
    }

    /// Updates the entry's logical last-used tick.
    pub fn record_use(&mut self, used_at_tick: u64) {
        self.last_used_at_tick = used_at_tick;
    }
}

const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

fn mix_usize(mut hash: u64, value: usize) -> u64 {
    for byte in value.to_le_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}
