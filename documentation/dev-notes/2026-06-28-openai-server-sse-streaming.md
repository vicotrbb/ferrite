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
Streaming responses are channel-backed: the HTTP response is created before the
blocking generation loop finishes, and token chunks are sent from the runtime
token callback as generation progresses.

## Implementation Notes

- Added `CompletionStreamChunk` and `ChatCompletionStreamChunk` schemas.
- Added a small `scalar_llama_chat_f32_gguf_fixture` in a separate
  `chat_llama` fixture module so chat-route tests can exercise real tokenizer
  and model loading without expanding the existing scalar fixture file.
- Runtime generation now records per-token decoded text pieces in
  `GeneratedText`, while preserving the full decoded completion text for
  non-streaming responses.
- Runtime generation exposes `generate_with_token_callback`, which the server
  streaming routes use to send each token chunk through a bounded SSE channel.
- `crates/ferrite-server/src/openai/streaming.rs` owns the channel-backed SSE
  sender so route handlers do not manually format event frames.

## Verification

Red tests first:

```sh
cargo test -p ferrite-server -- openai::routes_tests::completions_endpoint_streams_openai_sse_chunks openai::routes_tests::chat_endpoint_streams_openai_sse_chunks -- --nocapture
```

Initial result before implementation:

- both streaming tests failed with `501` instead of `200`.

Progressive-streaming red tests:

```sh
cargo test -p ferrite-server -- runtime::tests::generate_with_token_callback_reports_each_token_piece -- --nocapture
cargo test -p ferrite-server -- openai::streaming::tests::channel_response_streams_serialized_events_and_done -- --nocapture
cargo test -p ferrite-server -- openai::routes_tests::completion_stream_helper_emits_tokens_from_generation_callback -- --nocapture
```

Initial results before each implementation slice:

- runtime callback test failed because `generate_with_token_callback` did not
  exist.
- channel-backed SSE test failed because `channel_response` did not exist.
- route helper test failed because `completion_stream_response` did not exist.

Final verification:

```sh
cargo test -p ferrite-server -- --nocapture
cargo clippy -p ferrite-server -p ferrite-fixtures --all-targets -- -D warnings
cargo check --workspace
git diff --check
```

Observed result:

- `cargo test -p ferrite-server -- --nocapture`: 11 passed.
- `cargo clippy -p ferrite-server -p ferrite-fixtures --all-targets -- -D warnings`: passed.
- `cargo check --workspace`: passed.
- `git diff --check`: passed.
