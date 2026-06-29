# OpenAI Chat Request Validation Test Split

Date: 2026-06-29

## Scope

This slice moved top-level chat request validation tests from
`crates/ferrite-server/src/openai/unsupported_tests.rs` into the focused
`crates/ferrite-server/src/openai/chat_request_validation_tests.rs` module.

The moved tests cover:

- missing `model`;
- non-string `model`;
- missing `messages`;
- null `messages`;
- non-array `messages`;
- non-object message items.

This is a test-organization slice only. It does not change request parsing,
OpenAI-shaped error bodies, unsupported-field reporting, routing, or inference
execution.

## Verification

Before the move:

```sh
cargo test -p ferrite-server --lib openai::unsupported_tests:: -- --nocapture
```

Observed result:

```text
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 235 filtered out; finished in 0.01s
```

After the move:

```sh
cargo test -p ferrite-server --lib openai::chat_request_validation_tests:: -- --nocapture
cargo test -p ferrite-server --lib openai::unsupported_tests:: -- --nocapture
```

Observed results:

```text
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 249 filtered out; finished in 0.00s
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 241 filtered out; finished in 0.01s
```

`unsupported_tests.rs` now contains 287 lines, while
`chat_request_validation_tests.rs` contains 114 lines.
