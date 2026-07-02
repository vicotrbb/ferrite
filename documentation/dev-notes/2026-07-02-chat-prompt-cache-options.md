# Chat Prompt Cache Options

Date: 2026-07-02

## Slice

This slice maps OpenAI-compatible chat `prompt_cache_key` metadata into a
generic runtime cache-options value.

It is part of ADR 0009 phase 1. It does not perform cache lookup, K/V reuse,
entry insertion, or nonzero cached-token accounting.

## Implementation

- Added `GenerationCacheOptions` in
  `crates/ferrite-server/src/runtime/cache_options.rs`.
- Added `ChatCompletionRequest::cache_options`.
- Threaded chat cache options through:
  - non-streaming chat generation;
  - streaming chat generation;
  - runtime generation entry points.
- Kept completions on default cache options because the completions request
  schema does not currently accept `prompt_cache_key`.

The runtime type is intentionally generic. The OpenAI request field acts only as
an optional namespace for future cache keys; it is not proof that two prompts
share token identity.

## Red Test

The initial focused test failed before implementation with:

```text
error[E0599]: no method named `cache_options` found for struct `chat::ChatCompletionRequest`
```

## Validation

Focused checks:

```sh
CARGO_TARGET_DIR=target/codex-cache-options cargo test -p ferrite-server openai::schema::chat::tests::chat_request_maps_prompt_cache_key_to_generation_cache_namespace -- --nocapture
CARGO_TARGET_DIR=target/codex-cache-options cargo test -p ferrite-server openai::schema::chat::tests -- --nocapture
```

Package checks:

```sh
CARGO_TARGET_DIR=target/codex-cache-options cargo test -p ferrite-server --lib
cargo fmt --all -- --check
git diff --check
```

Results:

- Focused cache-options test: passed.
- Chat schema tests: 2 passed.
- `cargo test -p ferrite-server --lib`: 357 passed.
- `cargo fmt --all -- --check`: passed after running `cargo fmt --all`.
- `git diff --check`: passed.

## Environment Note

The shared target directory was locked by a background workspace
`cargo check --workspace`, so validation used `target/codex-cache-options`.

## Limits

This slice only creates and threads request cache metadata. It does not change
generation output, does not report nonzero `cached_tokens`, and does not retain
or reuse K/V state. The next cache slice should build the token-exact runtime
key from model, tokenizer, rendered prompt tokens, execution policy, and this
optional namespace before any reuse path is enabled.
