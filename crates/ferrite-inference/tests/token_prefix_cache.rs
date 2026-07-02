use ferrite_inference::prefix_cache::{PrefixCacheKey, TokenPrefixIdentity};

#[test]
fn token_prefix_identity_uses_token_order_and_length() {
    let first = TokenPrefixIdentity::from_tokens([1, 2, 3]);
    let same = TokenPrefixIdentity::from_tokens([1, 2, 3]);
    let reordered = TokenPrefixIdentity::from_tokens([1, 3, 2]);
    let extended = TokenPrefixIdentity::from_tokens([1, 2, 3, 0]);

    assert_eq!(first, same);
    assert_eq!(first.token_count(), 3);
    assert_eq!(first.tokens(), &[1, 2, 3]);
    assert_ne!(first, reordered);
    assert_ne!(first, extended);
}

#[test]
fn prefix_cache_key_includes_model_tokenizer_and_template_fingerprints() {
    let prefix = TokenPrefixIdentity::from_tokens([10, 20, 30]);
    let key = PrefixCacheKey::new("model-a", "tokenizer-a", "template-a", prefix.clone());

    assert_eq!(key.prefix_token_count(), 3);
    assert_ne!(
        key,
        PrefixCacheKey::new("model-b", "tokenizer-a", "template-a", prefix.clone())
    );
    assert_ne!(
        key,
        PrefixCacheKey::new("model-a", "tokenizer-b", "template-a", prefix.clone())
    );
    assert_ne!(
        key,
        PrefixCacheKey::new("model-a", "tokenizer-a", "template-b", prefix)
    );
}
