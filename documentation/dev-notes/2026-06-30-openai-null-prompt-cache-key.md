# OpenAI Null Prompt Cache Key

## Summary

Ferrite now treats explicit JSON `null` for the optional OpenAI-compatible chat
`prompt_cache_key` request field as neutral.

String prompt-cache keys remain accepted. Non-string non-null values remain
rejected with the existing OpenAI-shaped invalid request error path.

## TDD

The schema-level RED test was:

```text
cargo test -p ferrite-server --lib null_prompt_cache_key_is_valid -- --nocapture
```

It failed because `is_prompt_cache_key(&Some(Value::Null))` returned `false`.

The implementation changed the prompt-cache-key helper to treat `Value::Null`
like an omitted optional field.

## Validation

```text
cargo test -p ferrite-server --lib openai::schema::prompt_cache_key::tests -- --nocapture
cargo test -p ferrite-server --lib chat_endpoint_accepts_null_prompt_cache_key -- --nocapture
cargo test -p ferrite-server --lib chat_endpoint_accepts_prompt_cache_key -- --nocapture
cargo test -p ferrite-server --lib chat_endpoint_rejects_malformed_prompt_cache_key -- --nocapture
cargo fmt --all -- --check
git diff --check
```
