# OpenAI Inference Wait Timeout

Date: 2026-06-28

## Summary

Ferrite's OpenAI-compatible server now supports an explicit bounded wait before
returning inference backpressure. The default remains `0` ms, so overlapping
generation requests still receive the existing OpenAI-shaped `429` immediately
unless the operator opts into a wait window.

This improves local OpenAI client ergonomics without changing the core runtime
invariant: one request at a time may hold the model inference permit, and the
single loaded engine remains protected by the existing mutex.

## Implementation Notes

- Added `--inference-wait-ms` to `ferrite-server`.
- Added `ServerConfig::inference_wait_timeout()`.
- Added `ServerState::with_inference_wait_timeout()` and async permit
  acquisition with `tokio::time::timeout`.
- OpenAI completion and chat routes now await permit acquisition through the
  configured timeout before returning `rate_limit_error`.
- No unbounded request queue was added.

## TDD Evidence

Red tests first:

```sh
cargo test -p ferrite-server config::tests::parses_inference_wait_timeout -- --nocapture
cargo test -p ferrite-server openai::routes_tests::completions_endpoint_waits_for_busy_inference_within_configured_timeout -- --nocapture
```

Initial result:

- both commands failed to compile because `ServerConfig::inference_wait_timeout`
  and `ServerState::with_inference_wait_timeout` did not exist.

Final targeted verification:

```sh
cargo test -p ferrite-server config::tests::parses_inference_wait_timeout -- --nocapture
cargo test -p ferrite-server openai::routes_tests::completions_endpoint_waits_for_busy_inference_within_configured_timeout -- --nocapture
```

Observed result:

- config parser test passed.
- route test passed and returned a successful fixture completion after the held
  permit was released within the configured wait timeout.

## Package Verification

```sh
cargo fmt --all -- --check
git diff --check
cargo test -p ferrite-server -- --nocapture
cargo clippy -p ferrite-server --all-targets -- -D warnings
```

Observed result:

- `cargo fmt --all -- --check`: passed.
- `git diff --check`: passed.
- `cargo test -p ferrite-server -- --nocapture`: 50 unit tests, 7
  `async-openai` client integration tests, and 6 fixture live HTTP integration
  tests passed; 4 real Tier 0 HTTP tests and 5 real Tier 1 HTTP tests remained
  ignored by design.
- `cargo clippy -p ferrite-server --all-targets -- -D warnings`: passed.

## Remaining Limits

- This proves configured bounded waiting with the fixture HTTP path, not real
  Tier 1 successful concurrent serving.
- It does not make inference parallel. Requests still execute through one
  permit and the single loaded engine.
- The default `0` ms setting preserves immediate backpressure behavior.
