# OpenAI stream options test split

## Context

`crates/ferrite-server/src/openai/stream_options_tests.rs` had grown to 300
lines after the stream usage and obfuscation compatibility slices. That made it
harder to scan and conflicted with the repository preference for small, focused
Rust modules.

## Slice

Split the monolithic stream options test file into focused modules:

- `stream_usage_options_tests.rs`
- `stream_obfuscation_options_tests.rs`
- `stream_options_validation_tests.rs`
- `stream_options_test_support.rs`

No server behavior changed.

## Validation

Executed:

- `cargo test -p ferrite-server --lib openai::stream_usage_options_tests -- --nocapture`
- `cargo test -p ferrite-server --lib openai::stream_obfuscation_options_tests -- --nocapture`
- `cargo test -p ferrite-server --lib openai::stream_options_validation_tests -- --nocapture`

All focused module tests passed after the split.
