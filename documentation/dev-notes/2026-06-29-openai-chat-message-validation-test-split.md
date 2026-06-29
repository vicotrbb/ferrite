# OpenAI chat message validation test split

## Scope

This slice keeps Ferrite's OpenAI-compatible chat request validation coverage
intact while reducing `crates/ferrite-server/src/openai/unsupported_tests.rs`.

Message role, message metadata, and message content-part rejection cases now
live in `chat_message_validation_tests.rs`. The shared `post_chat_json` helper
now lives in `test_support.rs` so unsupported-request tests and message
validation tests use the same lightweight route harness.

## Evidence

- `unsupported_tests.rs` reduced from 738 lines to 399 lines.
- `chat_message_validation_tests.rs` contains 14 moved message validation tests.
- `test_support.rs` contains the shared `post_chat_json` response helper.
- `cargo test -p ferrite-server openai::chat_message_validation_tests -- --nocapture`
  passed: 14 passed.
- `cargo test -p ferrite-server openai::unsupported_tests -- --nocapture`
  passed: 20 passed.

## Boundary

This is a test-organization slice only. It does not change OpenAI request
validation behavior, response schemas, route behavior, or inference behavior.
