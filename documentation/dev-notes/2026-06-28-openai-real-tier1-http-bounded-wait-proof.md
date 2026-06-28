# OpenAI Real Tier 1 HTTP Bounded Wait Proof

Date: 2026-06-28

## Summary

Ferrite's OpenAI-compatible server now has an opt-in real Tier 1 proof for the
configured bounded inference wait path.

The test starts the local HTTP server with
`target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`, begins a longer streaming
chat request, then sends a concurrent legacy completion request while the first
request is expected to hold the single inference permit. With a 180 second wait
window configured, the second request waits and completes successfully instead
of receiving immediate backpressure.

## Implementation Notes

- Added `live_http_server_waits_for_concurrent_real_tier1_request` to
  `crates/ferrite-server/tests/openai_real_tier1_http.rs`.
- Added `LiveServer::start_with_existing_model_configured()` to test support so
  real-model HTTP tests can configure `ServerState` without duplicating server
  startup code.
- The production runtime still uses one inference permit and one loaded engine.
  This proof does not make inference parallel.

## TDD Evidence

Red command:

```sh
cargo test -p ferrite-server --test openai_real_tier1_http live_http_server_waits_for_concurrent_real_tier1_request -- --ignored --nocapture
```

Initial result:

- compile failed because
  `LiveServer::start_with_existing_model_configured` did not exist.

Green command:

```sh
cargo test -p ferrite-server --test openai_real_tier1_http live_http_server_waits_for_concurrent_real_tier1_request -- --ignored --nocapture
```

Observed result:

```text
test live_http_server_waits_for_concurrent_real_tier1_request has been running for over 60 seconds
test live_http_server_waits_for_concurrent_real_tier1_request ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 110.22s
```

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
  tests passed; 4 real Tier 0 HTTP tests and 6 real Tier 1 HTTP tests remained
  ignored by design.
- `cargo clippy -p ferrite-server --all-targets -- -D warnings`: passed.

## Remaining Limits

- This proves one successful waited overlap on the real Tier 1 Qwen2.5-0.5B
  Q4_K_M server path.
- It does not prove general concurrent serving throughput, queue fairness,
  multi-client saturation behavior, or broader Tier 1 model/prompt behavior.
- The default wait remains `0` ms, so immediate backpressure remains the default
  production posture.
