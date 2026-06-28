# OpenAI Unsupported Chat Fields

Date: 2026-06-28

## Summary

Ferrite's OpenAI-compatible chat endpoint now rejects unsupported request fields
that would otherwise imply capabilities the server does not implement.

Rejected fields currently include:

- `tools`
- `tool_choice`
- `parallel_tool_calls`
- `response_format`

The route returns an OpenAI-shaped `invalid_request_error` with status `400`
before prompt rendering or inference execution. This matches ADR 0008's rule
that unsupported OpenAI fields must not be silently ignored when honoring them
would be misleading.

## Implementation Notes

- Added `ChatCompletionRequest::unsupported_fields()` in the chat schema module.
- Added a route guard that converts unsupported field presence into a focused
  OpenAI error response.
- Added `crates/ferrite-server/src/openai/unsupported_tests.rs` so unsupported
  compatibility behavior does not expand the already-large route test file.

## Verification

Red tests first:

```sh
cargo test -p ferrite-server openai::unsupported_tests -- --nocapture
```

Initial result before implementation:

- `chat_endpoint_rejects_tool_fields` returned `503` instead of `400`.
- `chat_endpoint_rejects_response_format` returned `503` instead of `400`.

Final verification:

```sh
cargo test -p ferrite-server -- --nocapture
cargo clippy -p ferrite-server --all-targets -- -D warnings
cargo check --workspace
git diff --check
```

Observed result:

- `cargo test -p ferrite-server -- --nocapture`: 21 unit tests passed,
  `async_openai_client_uses_ferrite_base_url` passed, and
  `live_http_server_accepts_openai_style_chat_request` passed.
- `cargo clippy -p ferrite-server --all-targets -- -D warnings`: passed.
- `cargo check --workspace`: passed.
- `git diff --check`: passed.
