# OpenAI Bearer Auth

Date: 2026-06-28

## Summary

Ferrite's OpenAI-compatible server now supports an optional local bearer-token
policy:

```sh
ferrite-server --api-key local-secret
```

When configured, `/v1/*` routes require:

```http
Authorization: Bearer local-secret
```

`GET /health` remains unauthenticated so local readiness checks can keep working
without embedding a secret.

## Implementation Notes

- Added `--api-key` parsing to `ServerConfig`.
- Stored the optional API key on `ServerState`.
- Added a route-level guard for OpenAI `/v1/*` handlers.
- Added an OpenAI-shaped `authentication_error` response with HTTP `401`.
- Kept auth tests in `crates/ferrite-server/src/openai/auth_tests.rs` rather
  than growing the broader route-test file.

## Verification

Red tests first:

```sh
cargo test -p ferrite-server openai::auth_tests -- --nocapture
cargo test -p ferrite-server config::tests::parses_optional_api_key -- --nocapture
```

Initial result before implementation:

- compile failed because `ServerState::with_api_key` did not exist.
- compile failed because `ServerConfig::api_key` did not exist.

Final verification:

```sh
cargo fmt --all
cargo test -p ferrite-server -- --nocapture
cargo clippy -p ferrite-server --all-targets -- -D warnings
cargo check --workspace
git diff --check
```

Observed result:

- `cargo test -p ferrite-server -- --nocapture`: 25 unit tests passed,
  `async_openai_client_uses_ferrite_base_url` passed, and
  `live_http_server_accepts_openai_style_chat_request` passed.
- `cargo clippy -p ferrite-server --all-targets -- -D warnings`: passed after
  replacing disallowed test `expect_err` / `panic!` usage.
- `cargo check --workspace`: passed.
- `git diff --check`: passed.
