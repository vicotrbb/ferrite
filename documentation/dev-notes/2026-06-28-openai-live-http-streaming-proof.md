# OpenAI Live HTTP Streaming Proof

Date: 2026-06-28

## Summary

Ferrite's live HTTP integration coverage now includes curl-style raw HTTP
streaming requests for both OpenAI-compatible generation endpoints:

- `POST /v1/chat/completions`
- `POST /v1/completions`

The tests open a real TCP connection to a live Axum server, send HTTP/1.1 JSON
requests with `stream: true`, and verify OpenAI-style Server-Sent Events are
returned with `data: ...` chunks and `data: [DONE]`.

## Implementation Notes

- Added `live_http_server_streams_openai_style_chat_chunks` to
  `crates/ferrite-server/tests/openai_http.rs`.
- Added `live_http_server_streams_openai_style_legacy_completion_chunks` to
  `crates/ferrite-server/tests/openai_http.rs`.
- The proof checks the `text/event-stream` content type, OpenAI-shaped chunk
  objects, generated fixture text, and `[DONE]` terminator.
- No production code changed. This is HTTP compatibility evidence with the
  deterministic fixture model, not real-model inference evidence.

## Verification

Focused proof:

```sh
cargo test -p ferrite-server --test openai_http -- --nocapture
```

Observed result:

- 6 live HTTP tests passed.

Server verification before the test commit:

```sh
cargo fmt --all
cargo test -p ferrite-server -- --nocapture
cargo clippy -p ferrite-server --all-targets -- -D warnings
git diff --check
```

Observed result:

- `cargo test -p ferrite-server -- --nocapture`: 48 unit tests passed,
  7 `openai_client` integration tests passed, and 6 `openai_http` integration
  tests passed.
- `cargo clippy -p ferrite-server --all-targets -- -D warnings`: passed.
- `git diff --check`: passed.

## Remaining Proof Boundary

The OpenAI local-server surface now has raw live HTTP coverage for model list,
chat completions, legacy completions, and streaming. It still does not prove a
real GGUF model through the HTTP server; that requires a bounded real-model
server proof plan and should remain separate from fast fixture compatibility
tests.
