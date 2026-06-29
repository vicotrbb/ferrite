# OpenAI Stop Filter Module Split

Date: 2026-06-29

## Scope

This slice moved OpenAI stop-sequence filtering from
`crates/ferrite-server/src/openai/generation.rs` into the focused private
`crates/ferrite-server/src/openai/stop_filter.rs` module.

`generation.rs` now keeps the streaming and blocking generation orchestration,
while `stop_filter.rs` owns:

- non-streaming stop-sequence application;
- streaming visible-token filtering;
- retained suffix calculation for partial stop-sequence prefixes.

This is a production-code organization slice only. It does not change stop
matching semantics, SSE chunk ordering, finish reasons, routing, or inference
execution.

## Verification

Before the move:

```sh
cargo test -p ferrite-server --lib openai::generation::tests:: -- --nocapture
cargo test -p ferrite-server --lib openai::stop_sequences_tests:: -- --nocapture
cargo test -p ferrite-server --lib openai::routes_tests::completion_stream_helper_emits_tokens_from_generation_callback -- --nocapture
```

Observed results:

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 254 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 247 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 254 filtered out; finished in 0.00s
```

After the move:

```sh
cargo test -p ferrite-server --lib openai::stop_filter::tests:: -- --nocapture
cargo test -p ferrite-server --lib openai::stop_sequences_tests:: -- --nocapture
cargo test -p ferrite-server --lib openai::routes_tests::completion_stream_helper_emits_tokens_from_generation_callback -- --nocapture
```

Observed results:

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 254 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 247 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 254 filtered out; finished in 0.01s
```

`generation.rs` now contains 240 lines, while `stop_filter.rs` contains 122
lines.
