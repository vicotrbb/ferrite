# OpenAI Route Streaming Test Split

Date: 2026-06-29

## Context

`crates/ferrite-server/src/openai/routes_tests.rs` mixed non-streaming route
acceptance tests with SSE route and stream-helper coverage. As the
OpenAI-compatible HTTP surface grows, streaming behavior needs a focused test
home separate from ordinary request acceptance.

## Change

- Added `route_streaming_tests.rs` for completion SSE, chat SSE, and the
  completion stream helper callback path.
- Reduced `routes_tests.rs` to non-streaming fixture-backed completion and chat
  request acceptance tests.

## Validation

Baseline before the split:

```sh
cargo test -p ferrite-server --lib openai::routes_tests::completions_endpoint_streams_openai_sse_chunks -- --nocapture
cargo test -p ferrite-server --lib openai::routes_tests::chat_endpoint_streams_openai_sse_chunks -- --nocapture
cargo test -p ferrite-server --lib openai::routes_tests::completion_stream_helper_emits_tokens_from_generation_callback -- --nocapture
cargo test -p ferrite-server --lib openai::routes_tests -- --nocapture
```

After the split:

```sh
cargo test -p ferrite-server --lib openai::route_streaming_tests -- --nocapture
cargo test -p ferrite-server --lib openai::routes_tests -- --nocapture
```

Results:

- `route_streaming_tests`: 3 passed.
- `routes_tests`: 7 passed.
