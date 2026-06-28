# OpenAI Generation Module Refactor

Date: 2026-06-28

## Summary

Ferrite's OpenAI-compatible server route module no longer owns blocking
generation orchestration or streaming response construction.

This keeps HTTP routing, request validation, auth checks, and token-limit
normalization in `openai::routes`, while `openai::generation` owns:

- non-streaming blocking generation through `tokio::task::spawn_blocking`;
- SSE stream generation through `generate_with_token_callback`;
- chat and legacy completion stream response construction.

## Implementation Notes

- Added `crates/ferrite-server/src/openai/generation.rs`.
- Moved the existing generation helpers out of
  `crates/ferrite-server/src/openai/routes.rs`.
- Updated the focused stream-helper regression test to call the new module.
- No route names, schemas, status codes, auth behavior, backpressure behavior,
  or inference behavior changed.

The production `routes.rs` file shrank from 327 lines to 211 lines. The new
`generation.rs` module is 123 lines.

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

This is an organization and maintainability slice for the OpenAI-compatible
server. It does not add new endpoints, expand OpenAI API parity, or change
real-model correctness and throughput evidence.
