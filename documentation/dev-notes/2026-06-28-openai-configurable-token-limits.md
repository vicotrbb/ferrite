# OpenAI Configurable Token Limits

Date: 2026-06-28

## Summary

Ferrite's OpenAI-compatible server no longer hard-codes generation token limits
inside the route layer.

Token policy now lives in `crates/ferrite-server/src/limits.rs`, server config
parses `--default-max-tokens` and `--hard-max-tokens`, server state carries the
validated limits, and both `/v1/chat/completions` and `/v1/completions` normalize
request limits through the same policy.

## Implementation Notes

- Added `TokenLimits` with default and hard limits matching the previous route
  constants: 16 and 256.
- Added order-independent CLI parsing for `--default-max-tokens` and
  `--hard-max-tokens`.
- Added route coverage for configured defaults and configured hard-limit
  rejection.
- Kept OpenAI errors in the existing `invalid_request_error` response shape.

## Verification

Red tests first:

```sh
cargo test -p ferrite-server config::tests::parses_token_limits -- --nocapture
cargo test -p ferrite-server openai::routes_tests::chat_endpoint_uses_configured_default_max_tokens -- --nocapture
```

Initial result before implementation:

- compile failed because `crate::limits` did not exist.
- compile failed because `ServerConfig::token_limits` did not exist.
- compile failed because `ServerState::with_token_limits` did not exist.

Final verification:

```sh
cargo fmt --all
cargo test -p ferrite-server -- --nocapture
cargo clippy -p ferrite-server --all-targets -- -D warnings
cargo check --workspace
git diff --check
```

Observed result:

- `cargo test -p ferrite-server -- --nocapture`: 40 unit tests passed,
  2 `openai_client` integration tests passed, and 2 `openai_http` integration
  tests passed.
- `cargo clippy -p ferrite-server --all-targets -- -D warnings`: passed after
  replacing a new test `unwrap_err()` with an explicit match.
- `cargo check --workspace`: passed.
- `git diff --check`: passed.
