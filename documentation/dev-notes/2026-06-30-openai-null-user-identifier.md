# OpenAI Null User Identifier

## Summary

Ferrite now treats explicit JSON `null` for the optional OpenAI-compatible
`user` request field as neutral on both generation endpoints:

- `POST /v1/chat/completions`;
- `POST /v1/completions`.

This matches the existing request behavior where omitted `user` has no local
effect. String user identifiers remain accepted, and non-string non-null values
remain rejected with `error.param` set to `user`.

## TDD

The schema-level RED test was:

```text
cargo test -p ferrite-server --lib null_user_identifier_is_valid -- --nocapture
```

It failed because `is_user_identifier(&Some(Value::Null))` returned `false`.

The implementation changed the user-identifier helper to treat `Value::Null`
as valid, matching serde's route-level `Option<Value>` behavior for explicit
null request fields.

## Validation

```text
cargo test -p ferrite-server --lib openai::schema::user_identifier::tests -- --nocapture
cargo test -p ferrite-server --lib openai::user_identifier_tests -- --nocapture
cargo test -p ferrite-server --lib accepts_user_identifier -- --nocapture
cargo fmt --all -- --check
git diff --check
```
