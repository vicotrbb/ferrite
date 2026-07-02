# Runtime Prefix Cache Key

Date: 2026-07-02

## Slice

This slice prepares token-exact runtime prefix cache keys after real tokenizer
encoding in `ferrite-server`.

It does not implement cache lookup, K/V retention, session resume, non-zero
`cached_tokens`, or any performance behavior change.

## Implementation

- Added `crates/ferrite-server/src/runtime/prefix_cache.rs` for runtime cache
  key construction.
- Added model and tokenizer fingerprint strings to `InferenceEngine`.
- Derived conservative GGUF-content FNV-64 labels while loading the model:
  - `gguf-model-fnv64:{hash}`;
  - `gguf-tokenizer-fnv64:{hash}`.
- Built `PrefixCacheKey` after prompt tokenization, using the exact token IDs
  accepted by the runtime.
- Propagated `GenerationCacheOptions::namespace()` into the cache key.
- Kept the generated key unused for lookup in this slice to avoid claiming
  cache reuse before session-owned K/V state exists.

## Red Tests

The first focused runtime test failed before implementation because
`InferenceEngine::prefix_cache_key_for_prompt` did not exist.

After adding the helper, the test expectation was corrected from token ID `1`
to token ID `2`. The fixture tokenizer is `["<unk>", "hello", "winner"]`, so
the prompt `"winner"` must encode to `[2]`.

## Validation

Focused runtime check:

```sh
CARGO_TARGET_DIR=target/codex-runtime-cache-key cargo test -p ferrite-server runtime::tests::prefix_cache_key_uses_tokenized_prompt_and_cache_namespace -- --nocapture
```

Result: 1 passed.

Prefix-cache identity regression check:

```sh
CARGO_TARGET_DIR=target/codex-runtime-cache-key cargo test -p ferrite-inference --test token_prefix_cache -- --nocapture
```

Result: 6 passed.

Server library check:

```sh
CARGO_TARGET_DIR=target/codex-runtime-cache-key cargo test -p ferrite-server --lib
```

Result: 358 passed.

Formatting and whitespace checks:

```sh
cargo fmt --all -- --check
git diff --check
```

Results:

- `cargo fmt --all -- --check`: passed after applying `cargo fmt --all`.
- `git diff --check`: passed.

## Limits

The runtime still evaluates every prompt from scratch. The cache key is a
fail-closed identity object only.

The FNV labels are conservative content fingerprints, not a final semantic GGUF
metadata fingerprint design. A later slice should split model weights,
tokenizer configuration, chat template, execution policy, and request shape
more precisely before enabling cache hits.

No real model benchmark was run in this slice.
