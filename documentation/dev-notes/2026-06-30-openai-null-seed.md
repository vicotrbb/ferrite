# OpenAI Null Seed

## Summary

Ferrite now treats explicit JSON `null` for the optional OpenAI-compatible
`seed` request field as neutral on both generation endpoints:

- `POST /v1/chat/completions`;
- `POST /v1/completions`.

Integer seeds remain accepted, and non-integer non-null seeds remain rejected.
Ferrite still does not implement seeded sampling semantics; this slice only
keeps a neutral optional request shape from failing local OpenAI-compatible
client calls.

## TDD

The schema-level RED test was:

```text
cargo test -p ferrite-server --lib null_seed_is_valid -- --nocapture
```

It failed because `is_seed(&Some(Value::Null))` returned `false`.

The implementation changed the seed helper to treat `Value::Null` as valid,
matching omitted `seed` behavior.

## Validation

```text
cargo test -p ferrite-server --lib openai::schema::seed::tests -- --nocapture
cargo test -p ferrite-server --lib openai::seed_tests -- --nocapture
cargo test -p ferrite-server --lib accepts_seed -- --nocapture
cargo test -p ferrite-server --lib rejects_malformed_seed -- --nocapture
cargo fmt --all -- --check
git diff --check
```
