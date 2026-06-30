# OpenAI Null Metadata

## Summary

Ferrite now treats explicit JSON `null` for the optional OpenAI-compatible
chat `metadata` request field as neutral.

Object metadata with string keys and string values remains accepted. Non-object
metadata, non-string values, oversized metadata, and overlong keys or values
remain rejected.

## TDD

The schema-level RED test was:

```text
cargo test -p ferrite-server --lib null_metadata_is_valid -- --nocapture
```

It failed because `is_valid_metadata(&Some(Value::Null))` returned `false`.

The implementation changed the metadata helper to treat `Value::Null` like an
omitted optional metadata field.

## Validation

```text
cargo test -p ferrite-server --lib openai::schema::metadata::tests -- --nocapture
cargo test -p ferrite-server --lib chat_endpoint_accepts_null_metadata -- --nocapture
cargo test -p ferrite-server --lib chat_endpoint_accepts_metadata_object -- --nocapture
cargo test -p ferrite-server --lib chat_endpoint_rejects_malformed_metadata -- --nocapture
cargo fmt --all -- --check
git diff --check
```
