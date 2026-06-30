# OpenAI Null Safety Identifier

## Summary

Ferrite now treats explicit JSON `null` for the optional OpenAI-compatible chat
`safety_identifier` request field as neutral.

String safety identifiers within the existing local length limit remain
accepted. Overlong strings and non-string non-null values remain rejected.

## TDD

The schema-level RED test was:

```text
cargo test -p ferrite-server --lib null_safety_identifier_is_valid -- --nocapture
```

It failed because `is_safety_identifier(&Some(Value::Null))` returned `false`.

The implementation changed the safety-identifier helper to treat `Value::Null`
like an omitted optional field.

## Validation

```text
cargo test -p ferrite-server --lib openai::schema::safety_identifier::tests -- --nocapture
cargo test -p ferrite-server --lib chat_endpoint_accepts_null_safety_identifier -- --nocapture
cargo test -p ferrite-server --lib chat_endpoint_accepts_safety_identifier -- --nocapture
cargo test -p ferrite-server --lib chat_endpoint_rejects_malformed_safety_identifier -- --nocapture
cargo test -p ferrite-server --lib chat_endpoint_rejects_overlong_safety_identifier -- --nocapture
cargo fmt --all -- --check
git diff --check
```
