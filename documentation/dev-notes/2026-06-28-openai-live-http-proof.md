# OpenAI Live HTTP Proof

Date: 2026-06-28

## Summary

Ferrite's OpenAI-compatible server now has a live HTTP integration test that
starts the real Axum server on a loopback ephemeral port and sends an
OpenAI-style chat completion request over a TCP socket.

The request uses:

- `POST /v1/chat/completions`
- `Authorization: Bearer local-test`
- `Content-Type: application/json`
- `max_completion_tokens`

This proves the local-server path used by OpenAI-compatible clients is not only
covered through in-process router tests.

## Implementation Notes

- Added `crates/ferrite-server/tests/openai_http.rs` as a focused integration
  test file.
- Enabled Tokio's `io-util` feature for deterministic raw socket reads and
  writes.
- Avoided adding a large HTTP client dev dependency; the test speaks the small
  HTTP subset it needs directly.

## Verification

Focused run:

```sh
cargo test -p ferrite-server --test openai_http -- --nocapture
```

Initial results during test construction:

- the first request emitted no response because formatted header lines kept
  leading indentation spaces and Hyper closed the connection.
- after fixing header formatting, waiting for EOF hung because the server kept
  the socket reusable; the test now reads the JSON response body by
  `Content-Length`.

Final verification:

```sh
cargo test -p ferrite-server -- --nocapture
cargo clippy -p ferrite-server --all-targets -- -D warnings
cargo check --workspace
git diff --check
```

Observed result:

- `cargo test -p ferrite-server -- --nocapture`: 19 unit tests passed and
  `live_http_server_accepts_openai_style_chat_request` passed.
- `cargo clippy -p ferrite-server --all-targets -- -D warnings`: passed.
- `cargo check --workspace`: passed.
- `git diff --check`: passed.
