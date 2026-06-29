# OpenAI Token Limit Test Split

Date: 2026-06-29

## Scope

This slice moved chat token-limit route tests from
`crates/ferrite-server/src/openai/routes_tests.rs` into the focused
`crates/ferrite-server/src/openai/token_limit_tests.rs` module.

The moved tests cover:

- `max_completion_tokens` limiting for non-streaming chat completions;
- configured default max-token limits;
- configured hard max-token rejection;
- `max_completion_tokens` limiting for streaming chat completions.

This is a test-organization slice only. It does not change request schemas,
response schemas, routing, token-limit semantics, model loading, or inference
execution.

## Verification

Before the move:

```sh
cargo test -p ferrite-server --lib openai::routes_tests::chat_endpoint_honors_max_completion_tokens -- --nocapture
cargo test -p ferrite-server --lib openai::routes_tests::chat_endpoint_uses_configured_default_max_tokens -- --nocapture
cargo test -p ferrite-server --lib openai::routes_tests::chat_endpoint_rejects_configured_hard_max_tokens -- --nocapture
cargo test -p ferrite-server --lib openai::routes_tests::chat_stream_endpoint_honors_max_completion_tokens -- --nocapture
cargo test -p ferrite-server --lib openai::token_limit_tests:: -- --nocapture
```

Observed results:

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 254 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 254 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 254 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 254 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 253 filtered out; finished in 0.00s
```

After the move:

```sh
cargo test -p ferrite-server --lib openai::token_limit_tests:: -- --nocapture
cargo test -p ferrite-server --lib openai::routes_tests:: -- --nocapture
```

Observed results:

```text
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 249 filtered out; finished in 0.00s
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 243 filtered out; finished in 0.03s
```

`routes_tests.rs` now contains 318 lines, while `token_limit_tests.rs`
contains 192 lines.
