# OpenAI Chat Metadata Validation Test Split

Date: 2026-06-29

## Scope

This slice moved chat metadata and client identifier validation tests from
`crates/ferrite-server/src/openai/unsupported_tests.rs` into the focused
`crates/ferrite-server/src/openai/chat_metadata_validation_tests.rs` module.

The moved tests cover:

- malformed `metadata`;
- malformed `prompt_cache_key`;
- malformed `safety_identifier`;
- overlong `safety_identifier`;
- malformed `seed`.

This is a test-organization slice only. It does not change chat request
parsing, metadata validation, client identifier validation, OpenAI-shaped error
bodies, routing, or inference execution.

## Verification

Before the move:

```sh
cargo test -p ferrite-server --lib openai::unsupported_tests::chat_endpoint_rejects_malformed_metadata -- --nocapture
cargo test -p ferrite-server --lib openai::unsupported_tests::chat_endpoint_rejects_malformed_prompt_cache_key -- --nocapture
cargo test -p ferrite-server --lib openai::unsupported_tests::chat_endpoint_rejects_malformed_safety_identifier -- --nocapture
cargo test -p ferrite-server --lib openai::unsupported_tests::chat_endpoint_rejects_overlong_safety_identifier -- --nocapture
cargo test -p ferrite-server --lib openai::unsupported_tests::chat_endpoint_rejects_malformed_seed -- --nocapture
cargo test -p ferrite-server --lib openai::unsupported_tests:: -- --nocapture
```

Observed results:

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 254 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 254 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 254 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 254 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 254 filtered out; finished in 0.00s
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 241 filtered out; finished in 0.01s
```

After the move:

```sh
cargo test -p ferrite-server --lib openai::chat_metadata_validation_tests:: -- --nocapture
cargo test -p ferrite-server --lib openai::unsupported_tests:: -- --nocapture
```

Observed results:

```text
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 250 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 246 filtered out; finished in 0.01s
```

`unsupported_tests.rs` now contains 183 lines, while
`chat_metadata_validation_tests.rs` contains 106 lines.
