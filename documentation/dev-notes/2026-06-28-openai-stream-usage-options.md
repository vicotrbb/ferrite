# OpenAI Stream Usage Options

Date: 2026-06-28

## Summary

Ferrite now accepts `stream_options: {"include_usage": true}` on streaming
OpenAI-compatible chat and completion requests.

When usage is requested, the server streams normal token chunks, then a final
OpenAI-shaped chunk with `choices: []` and `usage`, then `data: [DONE]`.

## Implementation Notes

- Added `crates/ferrite-server/src/openai/schema/stream_options.rs` for typed
  `stream_options` parsing.
- Kept unknown nested stream options on the unsupported-field path.
- Added optional `usage` fields to chat and completion stream chunks.
- Used the existing `GeneratedText` returned by the runtime callback path to
  build the final usage chunk after token streaming completes.
- Added focused coverage in `crates/ferrite-server/src/openai/stream_options_tests.rs`
  instead of growing the general route test module.

## Verification

Red tests first:

```sh
cargo test -p ferrite-server openai::routes_tests::chat_stream_endpoint_emits_usage_when_requested -- --nocapture
cargo test -p ferrite-server openai::routes_tests::completions_stream_endpoint_emits_usage_when_requested -- --nocapture
```

Initial result before implementation:

- both requests returned `400` because `stream_options` was still rejected as
  unsupported.

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
  2 focused stream-options tests passed.
- `cargo test -p ferrite-server -- --nocapture`: 42 unit tests passed,
  2 `openai_client` integration tests passed, and 2 `openai_http` integration
  tests passed.
- `cargo clippy -p ferrite-server --all-targets -- -D warnings`: passed.
- `cargo check --workspace`: passed.
- `git diff --check`: passed.
