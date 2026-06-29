# OpenAI Health Test Split

Date: 2026-06-29

## Scope

This slice extracted the `/health` route tests from
`crates/ferrite-server/src/openai/routes_tests.rs` into
`crates/ferrite-server/src/openai/health_tests.rs`.

This is a test-organization slice only. It does not change server routing,
request schemas, response schemas, model loading, or inference behavior.

## Motivation

The OpenAI-compatible HTTP server tests have accumulated endpoint, validation,
streaming, and backpressure coverage. Keeping `/health` readiness checks in a
focused module makes the route test surface easier to scan and keeps future
OpenAI compatibility work from growing another broad test file.

## Verification

Before the split, the health tests passed in their original module:

```sh
cargo test -p ferrite-server --lib openai::routes_tests::health_endpoint -- --nocapture
```

Observed result:

```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 253 filtered out; finished in 0.01s
```

After the split:

```sh
cargo test -p ferrite-server --lib openai::health_tests:: -- --nocapture
cargo test -p ferrite-server --lib openai::routes_tests:: -- --nocapture
```

Observed results:

```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 253 filtered out; finished in 0.01s
test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 228 filtered out; finished in 0.03s
```

`routes_tests.rs` now contains 661 lines, while the new focused
`health_tests.rs` contains 43 lines.
