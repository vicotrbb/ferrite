#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TokenPrefixIdentity {
    tokens: Vec<usize>,
    token_hash: u64,
}

impl TokenPrefixIdentity {
    pub fn from_tokens(tokens: impl IntoIterator<Item = usize>) -> Self {
        let tokens = tokens.into_iter().collect::<Vec<_>>();
        let mut token_hash = FNV_OFFSET_BASIS;
        for token in &tokens {
            token_hash = mix_usize(token_hash, *token);
        }
        token_hash = mix_usize(token_hash, tokens.len());
        Self { tokens, token_hash }
    }

    pub fn token_count(&self) -> usize {
        self.tokens.len()
    }

    pub fn tokens(&self) -> &[usize] {
        &self.tokens
    }

    pub fn token_hash(&self) -> u64 {
        self.token_hash
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PrefixCacheFingerprints {
    model_fingerprint: String,
    tokenizer_fingerprint: String,
    template_fingerprint: String,
    execution_fingerprint: String,
    request_shape_fingerprint: String,
}

impl PrefixCacheFingerprints {
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
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PrefixCacheKey {
    fingerprints: PrefixCacheFingerprints,
    namespace: Option<String>,
    prefix: TokenPrefixIdentity,
}

impl PrefixCacheKey {
    pub fn new(fingerprints: PrefixCacheFingerprints, prefix: TokenPrefixIdentity) -> Self {
        Self {
            fingerprints,
            namespace: None,
            prefix,
        }
    }

    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    pub fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }

    pub fn prefix_token_count(&self) -> usize {
        self.prefix.token_count()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrefixCacheEntry {
    key: PrefixCacheKey,
    matched_prefix_token_count: usize,
    estimated_kv_bytes: u128,
    created_at_tick: u64,
    last_used_at_tick: u64,
}

impl PrefixCacheEntry {
    pub fn new(key: PrefixCacheKey, estimated_kv_bytes: u128, created_at_tick: u64) -> Self {
        Self {
            matched_prefix_token_count: key.prefix_token_count(),
            key,
            estimated_kv_bytes,
            created_at_tick,
            last_used_at_tick: created_at_tick,
        }
    }

    pub fn key(&self) -> &PrefixCacheKey {
        &self.key
    }

    pub fn matched_prefix_token_count(&self) -> usize {
        self.matched_prefix_token_count
    }

    pub fn estimated_kv_bytes(&self) -> u128 {
        self.estimated_kv_bytes
    }

    pub fn created_at_tick(&self) -> u64 {
        self.created_at_tick
    }

    pub fn last_used_at_tick(&self) -> u64 {
        self.last_used_at_tick
    }

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
