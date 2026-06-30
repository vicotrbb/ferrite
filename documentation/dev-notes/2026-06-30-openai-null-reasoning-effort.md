# OpenAI Null Reasoning Effort

## Summary

Ferrite now treats explicit JSON `null` for the optional OpenAI-compatible chat
`reasoning_effort` request field as neutral.

The existing local behavior remains unchanged for other values: `"none"` is
accepted as a no-op, while enabled or malformed reasoning-effort values remain
rejected because Ferrite does not implement hosted reasoning-token behavior.

## TDD

The schema-level RED test was:

```text
cargo test -p ferrite-server --lib null_reasoning_effort_is_neutral -- --nocapture
```

It failed because `is_no_reasoning_effort(&Some(Value::Null))` returned
`false`.

The implementation changed the reasoning-effort helper to treat `Value::Null`
like an omitted optional field.

## Validation

```text
cargo test -p ferrite-server --lib openai::schema::reasoning_effort::tests -- --nocapture
cargo test -p ferrite-server --lib chat_endpoint_accepts_null_reasoning_effort -- --nocapture
cargo test -p ferrite-server --lib chat_endpoint_accepts_no_reasoning_effort -- --nocapture
cargo test -p ferrite-server --lib chat_endpoint_rejects_enabled_reasoning_effort -- --nocapture
cargo fmt --all -- --check
git diff --check
```
