# OpenAI Server SSE Streaming

Date: 2026-06-28

## Summary

Ferrite's OpenAI-compatible server now accepts `stream: true` for:

- `POST /v1/completions`
- `POST /v1/chat/completions`

The routes return Server-Sent Events with OpenAI-shaped JSON chunks followed by
`data: [DONE]`. The implementation keeps streaming concerns in
`crates/ferrite-server/src/openai/streaming.rs`, response schemas in focused
schema modules, and model execution in `crates/ferrite-server/src/runtime.rs`.

## Implementation Notes

- Added `CompletionStreamChunk` and `ChatCompletionStreamChunk` schemas.
- Added a small `scalar_llama_chat_f32_gguf_fixture` in a separate
  `chat_llama` fixture module so chat-route tests can exercise real tokenizer
  and model loading without expanding the existing scalar fixture file.
- Runtime generation now records per-token decoded text pieces in
  `GeneratedText`, while preserving the full decoded completion text for
  non-streaming responses.
- The current SSE route emits chunks after the scalar generation call returns.
  A later latency slice should move token production onto a channel so chunks
  are flushed while generation is still running.

## Verification

Red tests first:

```sh
cargo test -p ferrite-server -- openai::routes_tests::completions_endpoint_streams_openai_sse_chunks openai::routes_tests::chat_endpoint_streams_openai_sse_chunks -- --nocapture
```

Initial result before implementation:

- both streaming tests failed with `501` instead of `200`.

Final verification:

```sh
cargo test -p ferrite-server -- --nocapture
cargo clippy -p ferrite-server -p ferrite-fixtures --all-targets -- -D warnings
cargo check --workspace
git diff --check
```

Observed result:

- `cargo test -p ferrite-server -- --nocapture`: 8 passed.
- `cargo clippy -p ferrite-server -p ferrite-fixtures --all-targets -- -D warnings`: passed.
- `cargo check --workspace`: passed.
- `git diff --check`: passed.
