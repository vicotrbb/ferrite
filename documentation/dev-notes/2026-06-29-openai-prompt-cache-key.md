# OpenAI Prompt Cache Key

Date: 2026-06-29

## Slice

Ferrite's OpenAI-compatible chat endpoint now accepts `prompt_cache_key` when it
is a string. The official Chat Completions API documents this field as an
optional string used by OpenAI to improve prompt-cache hit rates and as a
replacement for the deprecated `user` field. Ferrite treats it as local request
metadata and does not pass it into the inference core.

Source reference:

- https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create

## Implementation

- Added `crates/ferrite-server/src/openai/schema/prompt_cache_key.rs` to keep
  prompt-cache key validation separate from chat request logic.
- Updated chat unsupported-field detection to accept missing
  `prompt_cache_key` or string `prompt_cache_key` values.
- Added a fixture-backed route test for a request with `prompt_cache_key`.
- Added a route-level rejection regression for non-string `prompt_cache_key`.

## Red Test

```sh
cargo test -p ferrite-server chat_endpoint_accepts_prompt_cache_key -- --nocapture
```

Failed before implementation with:

```text
unsupported chat completion field(s): prompt_cache_key
```

## Validation

```sh
cargo test -p ferrite-server chat_endpoint_accepts_prompt_cache_key -- --nocapture
cargo test -p ferrite-server openai::schema::prompt_cache_key -- --nocapture
cargo test -p ferrite-server chat_endpoint_rejects_malformed_prompt_cache_key -- --nocapture
```

All commands passed after implementation.

## Limits

This slice does not implement prompt caching, cache lookup, cache retention,
cache eviction, or any scheduling behavior tied to `prompt_cache_key`.
`prompt_cache_retention` remains unsupported because accepting it would imply a
cache retention behavior Ferrite does not yet provide.
