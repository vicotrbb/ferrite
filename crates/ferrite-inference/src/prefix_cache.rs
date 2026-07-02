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
pub struct PrefixCacheKey {
    model_fingerprint: String,
    tokenizer_fingerprint: String,
    template_fingerprint: String,
    prefix: TokenPrefixIdentity,
}

impl PrefixCacheKey {
    pub fn new(
        model_fingerprint: impl Into<String>,
        tokenizer_fingerprint: impl Into<String>,
        template_fingerprint: impl Into<String>,
        prefix: TokenPrefixIdentity,
    ) -> Self {
        Self {
            model_fingerprint: model_fingerprint.into(),
            tokenizer_fingerprint: tokenizer_fingerprint.into(),
            template_fingerprint: template_fingerprint.into(),
            prefix,
        }
    }

    pub fn prefix_token_count(&self) -> usize {
        self.prefix.token_count()
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
