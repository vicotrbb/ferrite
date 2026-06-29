# OpenAI Stream Generation Module Split

Date: 2026-06-29

## Scope

Split OpenAI-compatible SSE stream response construction and callback-driven
token streaming out of `generation.rs` into a focused private
`stream_generation` module.

This is an organization-only slice. It does not change OpenAI-compatible HTTP
routes, stream event shapes, stop-sequence semantics, usage chunks, inference
execution, or non-streaming generation behavior.

## Rationale

`generation.rs` mixed non-streaming generation orchestration with SSE response
construction and stream callback plumbing. Keeping stream-specific response
assembly in its own module makes the OpenAI server easier to review while
preserving a small non-streaming generation module.

## Change

- Added `crates/ferrite-server/src/openai/stream_generation.rs`.
- Moved `CompletionStreamOptions`, `ChatStreamOptions`,
  `completion_stream_response`, `chat_stream_response`, and the private stream
  callback helper into the new module.
- Left `generate_text` and `generate_texts` in `generation.rs`.
- Updated route imports and the direct stream helper test to use the new
  module.

## Baseline

Before the refactor, the focused behavior was covered with:

```sh
cargo test -p ferrite-server --lib openai::route_streaming_tests -- --nocapture
cargo test -p ferrite-server --lib openai::response_stream_shape_tests -- --nocapture
cargo test -p ferrite-server --lib openai::stop_sequences_tests -- --nocapture
cargo test -p ferrite-server --lib openai::response_shape_tests -- --nocapture
```

Observed result:

- `openai::route_streaming_tests`: 3 passed.
- `openai::response_stream_shape_tests`: 2 passed.
- `openai::stop_sequences_tests`: 8 passed.
- `openai::response_shape_tests`: 2 passed.

## Validation

Post-refactor validation:

```sh
cargo test -p ferrite-server --lib openai::route_streaming_tests -- --nocapture
cargo test -p ferrite-server --lib openai::response_stream_shape_tests -- --nocapture
cargo test -p ferrite-server --lib openai::stop_sequences_tests -- --nocapture
cargo test -p ferrite-server --lib openai::response_shape_tests -- --nocapture
cargo fmt --all -- --check
git diff --check
CARGO_BUILD_JOBS=2 cargo clippy -p ferrite-server --all-targets -- -D warnings
```

Observed result:

- `openai::route_streaming_tests`: 3 passed.
- `openai::response_stream_shape_tests`: 2 passed.
- `openai::stop_sequences_tests`: 8 passed.
- `openai::response_shape_tests`: 2 passed.
- Formatting check passed.
- Whitespace check passed.
- `ferrite-server` clippy passed with warnings denied.

## Limits

This slice does not add new endpoint coverage and does not rerun ignored
real-model GGUF HTTP suites.
