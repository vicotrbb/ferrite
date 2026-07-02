# Token-Prefix Identity

Date: 2026-07-02

## Slice

This slice implements the first no-behavior-change primitive from ADR 0009:
token-level prefix identity in `ferrite-inference`.

It does not implement cache lookup, K/V state retention, server request wiring,
or OpenAI `cached_tokens` changes.

## Implementation

- Added `crates/ferrite-inference/src/prefix_cache.rs`.
- Exported `ferrite_inference::prefix_cache`.
- Added `TokenPrefixIdentity`, which keeps the exact token vector as part of
  equality and exposes a stable token hash for future lookup or diagnostics.
- Added `PrefixCacheKey`, which combines model, tokenizer, template, and token
  prefix identity.
- Added integration tests in
  `crates/ferrite-inference/tests/token_prefix_cache.rs`.

The identity deliberately does not rely on a hash alone. Two prefixes compare
equal only when their token vectors are equal.

## Red Test

```sh
cargo test -p ferrite-inference --test token_prefix_cache -- --nocapture
```

Failed before implementation with:

```text
error[E0432]: unresolved import `ferrite_inference::prefix_cache`
```

## Validation

The focused test was rerun with an isolated target directory because another
repo-local Cargo process held the default build lock:

```sh
CARGO_TARGET_DIR=target/codex-token-prefix \
  cargo test -p ferrite-inference --test token_prefix_cache -- --nocapture
```

Result: 2 passed.

Broader package validation:

```sh
CARGO_TARGET_DIR=target/codex-token-prefix cargo test -p ferrite-inference --tests
cargo fmt --all -- --check
git diff --check
```

Results:

- `cargo test -p ferrite-inference --tests`: passed.
- `cargo fmt --all -- --check`: passed.
- `git diff --check`: passed.

## Limits

This is only phase 1 identity plumbing. The next code slice should add explicit
cache options and result metadata while still keeping generation behavior
unchanged.
