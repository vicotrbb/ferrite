# OpenAI Completion Concurrency Test Split

Date: 2026-06-29

## Scope

This slice moved completion endpoint concurrency and inference-permit tests from
`crates/ferrite-server/src/openai/routes_tests.rs` into the focused
`crates/ferrite-server/src/openai/completion_concurrency_tests.rs` module.

The moved tests cover:

- immediate `429 rate_limit_error` when the inference permit is busy;
- bounded waiting for a busy inference permit within the configured timeout.

This is a test-organization slice only. It does not change routing, queueing,
inference permit behavior, timeout behavior, response schemas, or inference
execution.

## Verification

Before the move:

```sh
cargo test -p ferrite-server --lib openai::routes_tests::completions_endpoint_returns_429_when_inference_is_busy -- --nocapture
cargo test -p ferrite-server --lib openai::routes_tests::completions_endpoint_waits_for_busy_inference_within_configured_timeout -- --nocapture
cargo test -p ferrite-server --lib openai::routes_tests:: -- --nocapture
```

Observed results:

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 254 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 254 filtered out; finished in 0.02s
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 243 filtered out; finished in 0.05s
```

After the move:

```sh
cargo test -p ferrite-server --lib openai::completion_concurrency_tests:: -- --nocapture
cargo test -p ferrite-server --lib openai::routes_tests:: -- --nocapture
```

Observed results:

```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 253 filtered out; finished in 0.03s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 245 filtered out; finished in 0.01s
```

`routes_tests.rs` now contains 256 lines, while
`completion_concurrency_tests.rs` contains 70 lines.
