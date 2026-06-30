# OpenAI Completion Response Schema Split

Date: 2026-06-29

## Scope

This slice moved non-streaming legacy completion response schemas from
`crates/ferrite-server/src/openai/schema/completion.rs` into the focused
`crates/ferrite-server/src/openai/schema/completion_response.rs` module.

The legacy completion request parser remains in `schema/completion.rs`. The
public schema re-export is preserved through
`crates/ferrite-server/src/openai/schema.rs`, so route code continues to import
`CompletionResponse` from the schema facade.

This is a schema-organization slice only. It does not change request parsing,
response JSON shape, response IDs, finish reasons, prompt echo behavior, usage
accounting, routing, streaming behavior, or inference execution.

## Baseline Verification

Before the move:

```sh
cargo test -p ferrite-server --lib openai::completion_option_tests -- --nocapture
cargo test -p ferrite-server --lib openai::completion_unsupported_tests -- --nocapture
cargo test -p ferrite-server --lib openai::response_shape_tests -- --nocapture
```

Observed results:

```text
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 252 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 251 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 259 filtered out; finished in 0.00s
```

## Post-Change Verification

After the move:

```sh
cargo test -p ferrite-server --lib openai::completion_option_tests -- --nocapture
cargo test -p ferrite-server --lib openai::completion_unsupported_tests -- --nocapture
cargo test -p ferrite-server --lib openai::response_shape_tests -- --nocapture
cargo test -p ferrite-server --lib openai::schema::completion_response::tests::completion_response_ids_are_unique_within_the_same_second -- --nocapture
```

Observed results:

```text
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 252 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 251 filtered out; finished in 0.01s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 259 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 260 filtered out; finished in 0.00s
```

## File Shape

Before this slice, `schema/completion.rs` contained 253 lines and mixed request
parsing with non-streaming response serialization.

After this slice, request parsing remains in `schema/completion.rs` and
non-streaming response serialization lives in `schema/completion_response.rs`.
