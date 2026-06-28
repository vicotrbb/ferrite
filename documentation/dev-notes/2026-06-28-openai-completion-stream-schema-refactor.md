# OpenAI Completion Stream Schema Refactor

Date: 2026-06-28

## Summary

Ferrite's OpenAI-compatible legacy completion schema is split into smaller,
focused modules. Non-streaming completion request and response types remain in
`completion.rs`, while legacy completion stream chunk/context types now live in
`completion_stream.rs`.

## Implementation Notes

- Added `crates/ferrite-server/src/openai/schema/completion_stream.rs`.
- Kept public exports stable through `crates/ferrite-server/src/openai/schema.rs`.
- Moved only the existing legacy completion streaming schema and context
  helpers.
- No JSON field names, object names, route behavior, or inference behavior
  changed.

The schema split reduced `completion.rs` from 252 lines to 141 lines. The new
`completion_stream.rs` module is 113 lines.

## Verification

Commands run before the refactor commit:

```sh
cargo fmt --all
cargo test -p ferrite-server -- --nocapture
cargo clippy -p ferrite-server --all-targets -- -D warnings
git diff --check
```

Observed result:

- `cargo test -p ferrite-server -- --nocapture`: 48 unit tests passed,
  7 `openai_client` integration tests passed, 6 `openai_http` integration
  tests passed, 4 real Tier 0 HTTP tests were ignored by default, and 4 real
  Tier 1 HTTP tests were ignored by default.
- `cargo clippy -p ferrite-server --all-targets -- -D warnings`: passed.
- `git diff --check`: passed.

## Boundary

This is an organization slice for the OpenAI-compatible server schema. It does
not add new OpenAI API features, change legacy completion stream serialization,
or expand real-model proof coverage.
