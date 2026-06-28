# OpenAI Stream Options Require Streaming

Date: 2026-06-28

## Summary

Ferrite now rejects `stream_options` on non-streaming OpenAI-compatible chat and
completion requests.

This avoids silently ignoring a streaming-only option on a normal JSON
completion request, which would make the server appear to honor behavior it did
not apply.

## Implementation Notes

- `ChatCompletionRequest::unsupported_fields()` now reports `stream_options`
  when the field is present and `stream` is false.
- `CompletionRequest::unsupported_fields()` applies the same rule.
- Existing nested stream-option validation is preserved when `stream` is true.
- Added focused tests in `openai::stream_options_tests` for chat and legacy
  completion requests.

## Verification

Red test first:

```sh
cargo test -p ferrite-server openai::stream_options_tests -- --nocapture
```

Initial result before implementation:

- non-streaming chat with `stream_options` returned `503` because the request
  reached inference instead of failing validation.
- non-streaming completion with `stream_options` returned `503` for the same
  reason.

Final verification:

```sh
cargo fmt --all
cargo test -p ferrite-server openai::stream_options_tests -- --nocapture
cargo test -p ferrite-server -- --nocapture
cargo clippy -p ferrite-server --all-targets -- -D warnings
cargo check --workspace
git diff --check
```

Observed result:

- `cargo test -p ferrite-server openai::stream_options_tests -- --nocapture`:
  6 focused tests passed.
- `cargo test -p ferrite-server -- --nocapture`: 46 unit tests passed,
  3 `openai_client` integration tests passed, and 2 `openai_http` integration
  tests passed.
- `cargo clippy -p ferrite-server --all-targets -- -D warnings`: passed.
- `cargo check --workspace`: passed.
- `git diff --check`: passed.
