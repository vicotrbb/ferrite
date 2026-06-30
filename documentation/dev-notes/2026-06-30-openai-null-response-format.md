# OpenAI Null Response Format

## Summary

Ferrite now treats explicit JSON `null` for the optional OpenAI-compatible chat
`response_format` request field as neutral.

`{"type":"text"}` remains accepted as the local text-only no-op shape. JSON
object and JSON schema response formats remain rejected because Ferrite does not
implement structured-output generation.

## TDD

The schema-level RED test was:

```text
cargo test -p ferrite-server --lib null_response_format_is_neutral -- --nocapture
```

It failed because `is_neutral_response_format(&Some(Value::Null))` returned
`false`.

The implementation changed the response-format helper to treat `Value::Null`
like an omitted optional field.

## Validation

```text
cargo test -p ferrite-server --lib openai::schema::response_format::tests -- --nocapture
cargo test -p ferrite-server --lib chat_endpoint_accepts_null_response_format -- --nocapture
cargo test -p ferrite-server --lib chat_endpoint_accepts_text_response_format -- --nocapture
cargo test -p ferrite-server --lib chat_endpoint_rejects_json_response_format -- --nocapture
cargo fmt --all -- --check
git diff --check
```
