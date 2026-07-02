use ferrite_inference::prefix_cache::{
    PrefixCacheEntry, PrefixCacheFingerprints, PrefixCacheKey, PrefixCacheMetadataStore,
    TokenPrefixIdentity,
};

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
    let key = PrefixCacheKey::new(
        PrefixCacheFingerprints::new(
            "model-a",
            "tokenizer-a",
            "template-a",
            "scalar-default",
            "chat-default",
        ),
        prefix.clone(),
    );

    assert_eq!(key.prefix_token_count(), 3);
    assert_eq!(key.prefix_tokens(), &[10, 20, 30]);
    assert_eq!(key.prefix_token_hash(), prefix.token_hash());
    assert_eq!(key.fingerprints().model(), "model-a");
    assert_eq!(key.fingerprints().tokenizer(), "tokenizer-a");
    assert_eq!(key.fingerprints().template(), "template-a");
    assert_eq!(key.fingerprints().execution(), "scalar-default");
    assert_eq!(key.fingerprints().request_shape(), "chat-default");
    assert_ne!(
        key,
        PrefixCacheKey::new(
            PrefixCacheFingerprints::new(
                "model-b",
                "tokenizer-a",
                "template-a",
                "scalar-default",
                "chat-default"
            ),
            prefix.clone()
        )
    );
    assert_ne!(
        key,
        PrefixCacheKey::new(
            PrefixCacheFingerprints::new(
                "model-a",
                "tokenizer-b",
                "template-a",
                "scalar-default",
                "chat-default"
            ),
            prefix.clone()
        )
    );
    assert_ne!(
        key,
        PrefixCacheKey::new(
            PrefixCacheFingerprints::new(
                "model-a",
                "tokenizer-a",
                "template-b",
                "scalar-default",
                "chat-default"
            ),
            prefix
        )
    );
}

#[test]
fn prefix_cache_key_includes_execution_request_shape_and_namespace() {
    let prefix = TokenPrefixIdentity::from_tokens([10, 20, 30]);
    let fingerprints = PrefixCacheFingerprints::new(
        "model-a",
        "tokenizer-a",
        "template-a",
        "scalar-default",
        "chat-default",
    );
    let key = PrefixCacheKey::new(fingerprints.clone(), prefix.clone())
        .with_namespace("tenant-a:prompt-1");

    assert_eq!(key.namespace(), Some("tenant-a:prompt-1"));
    assert_ne!(
        key,
        PrefixCacheKey::new(
            PrefixCacheFingerprints::new(
                "model-a",
                "tokenizer-a",
                "template-a",
                "q8-k-experimental",
                "chat-default"
            ),
            prefix.clone()
        )
        .with_namespace("tenant-a:prompt-1")
    );
    assert_ne!(
        key,
        PrefixCacheKey::new(
            PrefixCacheFingerprints::new(
                "model-a",
                "tokenizer-a",
                "template-a",
                "scalar-default",
                "chat-stop=END"
            ),
            prefix.clone()
        )
        .with_namespace("tenant-a:prompt-1")
    );
    assert_ne!(
        key,
        PrefixCacheKey::new(fingerprints.clone(), prefix.clone())
            .with_namespace("tenant-b:prompt-1")
    );
    assert_ne!(
        key,
        PrefixCacheKey::new(fingerprints, TokenPrefixIdentity::from_tokens([10, 20, 31]))
            .with_namespace("tenant-a:prompt-1")
    );
}

#[test]
fn prefix_cache_entry_records_token_count_bytes_and_use_ticks() {
    let key = PrefixCacheKey::new(
        PrefixCacheFingerprints::new(
            "model-a",
            "tokenizer-a",
            "template-a",
            "scalar-default",
            "chat-default",
        ),
        TokenPrefixIdentity::from_tokens([10, 20, 30]),
    );
    let mut entry = PrefixCacheEntry::new(key.clone(), 4096, 7);

    assert_eq!(entry.key(), &key);
    assert_eq!(entry.matched_prefix_token_count(), 3);
    assert_eq!(entry.estimated_kv_bytes(), 4096);
    assert_eq!(entry.created_at_tick(), 7);
    assert_eq!(entry.last_used_at_tick(), 7);

    entry.record_use(11);

    assert_eq!(entry.created_at_tick(), 7);
    assert_eq!(entry.last_used_at_tick(), 11);
}

#[test]
fn prefix_cache_store_evicts_least_recent_entry_by_count() {
    let mut store = PrefixCacheMetadataStore::new(2, 10_000);
    let first = prefix_cache_entry("first", 100, 1);
    let second = prefix_cache_entry("second", 100, 2);
    let third = prefix_cache_entry("third", 100, 3);

    assert!(store.is_empty());
    assert!(store.insert(first.clone()).is_empty());
    assert!(!store.is_empty());
    assert!(store.insert(second.clone()).is_empty());
    assert!(store.record_hit(first.key(), 10).is_some());

    let evicted = store.insert(third.clone());

    assert_eq!(evicted, vec![second.clone()]);
    assert_eq!(store.len(), 2);
    assert_eq!(store.estimated_kv_bytes(), 200);
    assert!(store.get(first.key()).is_some());
    assert!(store.get(second.key()).is_none());
    assert!(store.get(third.key()).is_some());
}

#[test]
fn prefix_cache_store_evicts_until_byte_budget_fits() {
    let mut store = PrefixCacheMetadataStore::new(4, 250);
    let first = prefix_cache_entry("first", 100, 1);
    let second = prefix_cache_entry("second", 100, 2);
    let third = prefix_cache_entry("third", 100, 3);

    assert!(store.insert(first.clone()).is_empty());
    assert!(store.insert(second.clone()).is_empty());

    let evicted = store.insert(third.clone());

    assert_eq!(evicted, vec![first]);
    assert_eq!(store.len(), 2);
    assert_eq!(store.estimated_kv_bytes(), 200);
    assert!(store.get(second.key()).is_some());
    assert!(store.get(third.key()).is_some());
}

fn prefix_cache_entry(namespace: &str, bytes: u128, tick: u64) -> PrefixCacheEntry {
    PrefixCacheEntry::new(
        PrefixCacheKey::new(
            PrefixCacheFingerprints::new(
                "model-a",
                "tokenizer-a",
                "template-a",
                "scalar-default",
                "chat-default",
            ),
            TokenPrefixIdentity::from_tokens([10, 20, tick as usize]),
        )
        .with_namespace(namespace),
        bytes,
        tick,
    )
}
