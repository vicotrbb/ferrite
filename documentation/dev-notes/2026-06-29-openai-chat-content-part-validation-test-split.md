# OpenAI Chat Content Part Validation Test Split

Date: 2026-06-29

## Scope

This slice moved chat content-part validation tests from
`crates/ferrite-server/src/openai/chat_message_validation_tests.rs` into the
focused
`crates/ferrite-server/src/openai/chat_content_part_validation_tests.rs`
module.

The moved tests cover:

- user messages containing refusal content parts;
- image content parts;
- audio content parts;
- malformed text content parts;
- non-string text content part payloads.

This is a test-organization slice only. It does not change chat message
parsing, content-part validation, OpenAI-shaped error bodies, routing, or
inference execution.

## Verification

Before the move:

```sh
cargo test -p ferrite-server --lib openai::chat_message_validation_tests::chat_endpoint_rejects_user_refusal_content_parts -- --nocapture
cargo test -p ferrite-server --lib openai::chat_message_validation_tests::chat_endpoint_rejects_image_content_parts -- --nocapture
cargo test -p ferrite-server --lib openai::chat_message_validation_tests::chat_endpoint_rejects_audio_content_parts -- --nocapture
cargo test -p ferrite-server --lib openai::chat_message_validation_tests::chat_endpoint_rejects_malformed_text_content_parts -- --nocapture
cargo test -p ferrite-server --lib openai::chat_message_validation_tests::chat_endpoint_rejects_non_string_text_content_parts -- --nocapture
cargo test -p ferrite-server --lib openai::chat_message_validation_tests:: -- --nocapture
```

Observed results:

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 254 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 254 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 254 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 254 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 254 filtered out; finished in 0.00s
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 241 filtered out; finished in 0.00s
```

After the move:

```sh
cargo test -p ferrite-server --lib openai::chat_content_part_validation_tests:: -- --nocapture
cargo test -p ferrite-server --lib openai::chat_message_validation_tests:: -- --nocapture
```

Observed results:

```text
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 250 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 246 filtered out; finished in 0.00s
```

`chat_message_validation_tests.rs` now contains 199 lines, while
`chat_content_part_validation_tests.rs` contains 116 lines.
