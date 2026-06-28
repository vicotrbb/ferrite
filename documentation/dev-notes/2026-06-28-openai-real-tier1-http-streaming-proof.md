# OpenAI Real Tier 1 HTTP Streaming Proof

Date: 2026-06-28

## Summary

Ferrite's opt-in real Tier 1 HTTP integration coverage now includes streaming
legacy completions.

The proof starts a live Axum server with
`target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`, sends a raw HTTP/1.1 request
to `POST /v1/completions` with `stream: true`, and verifies an OpenAI-style
Server-Sent Events response for the deterministic first generated token for
`hello world`.

## Implementation Notes

- Added `live_http_server_streams_with_real_tier1_model` to
  `crates/ferrite-server/tests/openai_real_tier1_http.rs`.
- The test remains ignored by default because it requires a local Tier 1 model
  artifact and loads a 379 MB GGUF file.
- No production server code changed.

## Verification

Explicit Tier 1 HTTP proof:

```sh
cargo test -p ferrite-server --test openai_real_tier1_http -- --ignored --nocapture
```

Observed result:

- 2 ignored Tier 1 real-model HTTP tests passed when explicitly enabled.
- Rust test harness time for the target: about 20.09s.

The new streaming test verified:

- HTTP status `200 OK`.
- `text/event-stream` content type.
- `text_completion` stream chunks.
- generated stream text `"\n"`.
- `data: [DONE]` terminator.

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
  tests passed, 4 real Tier 0 HTTP tests were ignored by default, and 2 real
  Tier 1 HTTP tests were ignored by default.
- `cargo clippy -p ferrite-server --all-targets -- -D warnings`: passed.
- `git diff --check`: passed.

## Boundary

This proves real Tier 1 Qwen2.5-0.5B Q4_K_M execution through both
non-streaming and streaming OpenAI-compatible legacy completions HTTP paths. It
does not prove Tier 1 chat, Tier 1 throughput through the server, or broader
Tier 1 model coverage.
