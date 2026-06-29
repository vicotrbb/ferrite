# OpenAI chat option test split

## Scope

This slice keeps Ferrite's OpenAI-compatible chat option coverage intact while
reducing the size of `crates/ferrite-server/src/openai/routes_tests.rs`.

The route-level test file had grown to cover health, catalog, completions, chat
request compatibility, streaming, and inference backpressure. The chat option
acceptance cases now live in `chat_option_tests.rs`, and common fixture/body
helpers live in `test_support.rs`.

## Evidence

- `routes_tests.rs` reduced from 1071 lines to 673 lines.
- `chat_option_tests.rs` contains the moved chat request option acceptance
  coverage.
- `test_support.rs` contains shared fixture and response body helpers for
  OpenAI route tests.
- `cargo test -p ferrite-server openai::chat_option_tests -- --nocapture`
  passed: 16 passed.
- `cargo test -p ferrite-server openai::routes_tests -- --nocapture` passed:
  28 passed.

## Boundary

This is a test-organization slice only. It does not change request schemas,
route behavior, inference behavior, or real-model coverage.
