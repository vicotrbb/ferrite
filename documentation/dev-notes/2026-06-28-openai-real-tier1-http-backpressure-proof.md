# OpenAI Real Tier 1 HTTP Backpressure Proof

Date: 2026-06-28

## Summary

Ferrite's opt-in real Tier 1 HTTP integration coverage now proves bounded
server backpressure with a real Qwen2.5-0.5B Q4_K_M model.

The proof starts a live Axum server with
`target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`, starts a longer streaming
chat request, then sends a concurrent legacy completion request. The second
request receives an OpenAI-shaped `429` response while the first request holds
the single inference permit.

## Implementation Notes

- Added `live_http_server_rejects_concurrent_real_tier1_request` to
  `crates/ferrite-server/tests/openai_real_tier1_http.rs`.
- The test remains ignored by default because it requires a local Tier 1 model
  artifact and intentionally runs a longer real-model request.
- No production server code changed.

## Verification

Targeted real Tier 1 proof:

```sh
cargo test -p ferrite-server --test openai_real_tier1_http live_http_server_rejects_concurrent_real_tier1_request -- --ignored --nocapture
```

Observed result:

- 1 ignored real Tier 1 HTTP backpressure test passed when explicitly enabled.
- Rust test harness time for the targeted test: about 107.01s.

Full real Tier 1 HTTP proof:

```sh
cargo test -p ferrite-server --test openai_real_tier1_http -- --ignored --nocapture
```

Observed result:

- 5 ignored real Tier 1 HTTP tests passed when explicitly enabled.
- Rust test harness time for the target: about 156.87s.

The new backpressure test verified:

- The first streaming chat request completed with HTTP `200 OK`.
- The first response emitted `data: [DONE]`.
- The concurrent second request returned HTTP `429 Too Many Requests`.
- The error object used `rate_limit_error`.
- The error message was `inference request queue is full; retry later`.

Default server verification before the test commit:

```sh
cargo fmt --all
cargo test -p ferrite-server -- --nocapture
cargo clippy -p ferrite-server --all-targets -- -D warnings
git diff --check
```

Observed result:

- `cargo test -p ferrite-server -- --nocapture`: 48 unit tests passed,
  7 `openai_client` integration tests passed, 6 `openai_http` integration
  tests passed, 4 real Tier 0 HTTP tests were ignored by default, and 5 real
  Tier 1 HTTP tests were ignored by default.
- `cargo clippy -p ferrite-server --all-targets -- -D warnings`: passed.
- `git diff --check`: passed.

## Boundary

This proves real Tier 1 backpressure behavior for concurrent HTTP requests on
the local Qwen2.5-0.5B Q4_K_M server path. It does not prove concurrent
successful serving, queueing, batching, throughput under load, or broader model
coverage.
