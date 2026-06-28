# OpenAI Real Tier 0 HTTP Streaming Proof

Date: 2026-06-28

## Summary

Ferrite's opt-in real-model HTTP integration coverage now includes streaming
legacy completions with a real Tier 0 GGUF model.

The proof starts a live Axum server with
`target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf`, sends a raw HTTP/1.1 request
to `POST /v1/completions` with `stream: true`, and verifies an OpenAI-style
Server-Sent Events response that contains the deterministic first generated
token for `hello world`.

## Implementation Notes

- Added `live_http_server_streams_with_real_tier0_model` to
  `crates/ferrite-server/tests/openai_real_model_http.rs`.
- Shared the existing real-model path resolution through `real_model_path()`.
- The test remains ignored by default because it requires a local GGUF artifact
  and loads the real model.
- No production server code changed.

## Verification

Explicit real-model proof:

```sh
cargo test -p ferrite-server --test openai_real_model_http -- --ignored --nocapture
```

Observed result:

- 2 ignored real-model HTTP tests passed when explicitly enabled.
- The new streaming test verified:
  - HTTP status `200 OK`.
  - `text/event-stream` content type.
  - `text_completion` stream chunks.
  - generated stream text `"."`.
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
  tests passed, and 2 real-model HTTP tests were ignored by default.
- `cargo clippy -p ferrite-server --all-targets -- -D warnings`: passed.
- `git diff --check`: passed.

## Boundary

This proves real Tier 0 GGUF legacy completions through both non-streaming and
streaming OpenAI-compatible HTTP paths. It does not yet prove real GGUF chat
completions, real chat streaming, or Tier 1+ server behavior.
