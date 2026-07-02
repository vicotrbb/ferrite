# Token-Prefix Identity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the first no-behavior-change prefix-cache primitive: stable
token-prefix identity owned by `ferrite-inference`.

**Architecture:** Create a focused `prefix_cache` module in
`ferrite-inference` for token-level identity only. It must not store K/V state,
wire into server requests, or change generation behavior.

**Tech Stack:** Rust standard library, existing `ferrite-inference` crate,
Cargo integration tests.

---

### Task 1: Token Prefix Identity

**Files:**

- Create: `crates/ferrite-inference/src/prefix_cache.rs`
- Modify: `crates/ferrite-inference/src/lib.rs`
- Test: `crates/ferrite-inference/tests/token_prefix_cache.rs`

- [ ] **Step 1: Write the failing tests**

```rust
use ferrite_inference::prefix_cache::{PrefixCacheKey, TokenPrefixIdentity};

#[test]
fn token_prefix_identity_uses_token_order_and_length() {
    let first = TokenPrefixIdentity::from_tokens([1, 2, 3]);
    let same = TokenPrefixIdentity::from_tokens([1, 2, 3]);
    let reordered = TokenPrefixIdentity::from_tokens([1, 3, 2]);
    let extended = TokenPrefixIdentity::from_tokens([1, 2, 3, 0]);

    assert_eq!(first, same);
    assert_eq!(first.token_count(), 3);
    assert_ne!(first, reordered);
    assert_ne!(first, extended);
}

#[test]
fn prefix_cache_key_includes_model_tokenizer_and_template_fingerprints() {
    let prefix = TokenPrefixIdentity::from_tokens([10, 20, 30]);
    let key = PrefixCacheKey::new("model-a", "tokenizer-a", "template-a", prefix);

    assert_eq!(key.prefix_token_count(), 3);
    assert_ne!(
        key,
        PrefixCacheKey::new("model-b", "tokenizer-a", "template-a", prefix)
    );
    assert_ne!(
        key,
        PrefixCacheKey::new("model-a", "tokenizer-b", "template-a", prefix)
    );
    assert_ne!(
        key,
        PrefixCacheKey::new("model-a", "tokenizer-a", "template-b", prefix)
    );
}
```

- [ ] **Step 2: Run tests to verify RED**

Run:

```sh
cargo test -p ferrite-inference --test token_prefix_cache -- --nocapture
```

Expected: fail because `ferrite_inference::prefix_cache` does not exist.

- [ ] **Step 3: Implement minimal identity types**

Add `pub mod prefix_cache;` to `crates/ferrite-inference/src/lib.rs`.

Create `crates/ferrite-inference/src/prefix_cache.rs` with:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TokenPrefixIdentity {
    token_count: usize,
    token_hash: u64,
}

impl TokenPrefixIdentity {
    pub fn from_tokens(tokens: impl IntoIterator<Item = usize>) -> Self {
        let mut token_count = 0;
        let mut token_hash = FNV_OFFSET_BASIS;
        for token in tokens {
            token_count += 1;
            token_hash = mix_usize(token_hash, token);
        }
        token_hash = mix_usize(token_hash, token_count);
        Self {
            token_count,
            token_hash,
        }
    }

    pub fn token_count(&self) -> usize {
        self.token_count
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
```

- [ ] **Step 4: Run focused tests to verify GREEN**

Run:

```sh
cargo test -p ferrite-inference --test token_prefix_cache -- --nocapture
```

Expected: pass.

- [ ] **Step 5: Run package validation**

Run:

```sh
cargo test -p ferrite-inference --tests
cargo fmt --all -- --check
git diff --check
```

Expected: all pass.

- [ ] **Step 6: Commit**

```sh
git add crates/ferrite-inference/src/lib.rs \
  crates/ferrite-inference/src/prefix_cache.rs \
  crates/ferrite-inference/tests/token_prefix_cache.rs
git commit -m "feat: add token-prefix cache identity"
```
