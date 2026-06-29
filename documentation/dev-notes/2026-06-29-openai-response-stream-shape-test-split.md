# OpenAI Response Stream Shape Test Split

Date: 2026-06-29

## Context

`response_shape_tests.rs` mixed ordinary JSON response-shape checks with SSE
stream response-shape checks and carried local fixture plumbing that duplicated
the shared OpenAI test support helpers.

## Change

- Added `response_stream_shape_tests.rs` for streamed completion and chat
  response-shape assertions.
- Added `response_shape_assertions.rs` for shared shape assertions used by
  streamed and non-streamed response tests.
- Updated `response_shape_tests.rs` to use the shared test-support fixture and
  body helpers.

## Validation

Baseline before the split:

```sh
cargo test -p ferrite-server --lib openai::response_shape_tests::completions_stream_endpoint_returns_openai_choice_shape -- --nocapture
cargo test -p ferrite-server --lib openai::response_shape_tests::chat_stream_endpoint_returns_openai_choice_shape -- --nocapture
cargo test -p ferrite-server --lib openai::response_shape_tests -- --nocapture
```

After the split:

```sh
cargo test -p ferrite-server --lib openai::response_stream_shape_tests -- --nocapture
cargo test -p ferrite-server --lib openai::response_shape_tests -- --nocapture
```

Results:

- `response_stream_shape_tests`: 2 passed.
- `response_shape_tests`: 2 passed.
