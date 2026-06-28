# OpenAI Chat Max Completion Tokens

Date: 2026-06-28

## Summary

Ferrite's OpenAI-compatible chat endpoint now accepts
`max_completion_tokens` on `POST /v1/chat/completions`.

The field is treated as a compatibility alias for the server's generation token
limit. If both `max_tokens` and `max_completion_tokens` are present,
`max_tokens` keeps precedence to preserve the existing Ferrite request behavior.

## Implementation Notes

- Added `max_completion_tokens` to `ChatCompletionRequest`.
- Kept the alias local to the chat schema so route handlers and runtime
  generation continue to consume one `max_tokens()` accessor.
- Covered both non-streaming and `stream: true` chat responses with route tests
  using the real fixture-backed inference path.

## Verification

Red tests first:

```sh
cargo test -p ferrite-server -- openai::routes_tests::chat_endpoint_honors_max_completion_tokens openai::routes_tests::chat_stream_endpoint_honors_max_completion_tokens -- --nocapture
```

Initial result before implementation:

- non-streaming chat returned 16 repeated `winner` token pieces instead of one.
- streaming chat emitted 16 `winner` delta chunks instead of one.

Final verification:

```sh
cargo test -p ferrite-server -- --nocapture
cargo clippy -p ferrite-server --all-targets -- -D warnings
cargo check --workspace
git diff --check
```

Observed result:

- `cargo test -p ferrite-server -- --nocapture`: 17 passed.
- `cargo clippy -p ferrite-server --all-targets -- -D warnings`: passed.
- `cargo check --workspace`: passed.
- `git diff --check`: passed.
