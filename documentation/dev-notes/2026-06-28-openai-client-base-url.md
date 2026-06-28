# OpenAI Client Base URL Proof

Date: 2026-06-28

## Summary

Ferrite's OpenAI-compatible server now has a live integration test using
`async-openai` configured with a Ferrite base URL:

```text
http://127.0.0.1:<ephemeral-port>/v1
```

The test starts the real Axum server, loads the chat fixture model, creates an
`async-openai` client with a dummy API key, and calls `POST
/v1/chat/completions` through the client's typed Chat Completions API.

This closes the explicit ADR 0008 / goal-prompt proof gap requiring at least one
standard OpenAI client configured against Ferrite as the base URL.

## Implementation Notes

- Added `crates/ferrite-server/tests/openai_client.rs`.
- Extracted shared live-server fixture setup into
  `crates/ferrite-server/tests/support/mod.rs`.
- Kept `async-openai` as a dev dependency with `default-features = false` and
  only the `chat-completion` feature enabled. The test talks to local HTTP, so
  it does not need TLS features.
- Preserved the raw TCP HTTP smoke test in
  `crates/ferrite-server/tests/openai_http.rs`.

## Verification

Focused client proof:

```sh
cargo test -p ferrite-server --test openai_client -- --nocapture
```

Development result:

- first run failed at compile time because the typed chat symbols are exposed
  under `async_openai::types::chat`, not directly under
  `async_openai::types`.
- after the import fix, `async_openai_client_uses_ferrite_base_url` passed.
- dropping `async-openai`'s `rustls` feature preserved the passing local HTTP
  test and reduced unnecessary dev-dependency surface.

Final verification:

```sh
cargo test -p ferrite-server -- --nocapture
cargo clippy -p ferrite-server --all-targets -- -D warnings
cargo check --workspace
git diff --check
```

Observed result:

- `cargo test -p ferrite-server -- --nocapture`: 19 unit tests passed,
  `async_openai_client_uses_ferrite_base_url` passed, and
  `live_http_server_accepts_openai_style_chat_request` passed.
- `cargo clippy -p ferrite-server --all-targets -- -D warnings`: passed.
- `cargo check --workspace`: passed.
- `git diff --check`: passed.

Reference:

- `async-openai` 0.41.1 README describes OpenAI-compatible providers and
  configurable API base URLs: <https://docs.rs/async-openai/0.41.1>
