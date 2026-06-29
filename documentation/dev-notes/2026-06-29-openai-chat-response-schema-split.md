# OpenAI Chat Response Schema Split

Date: 2026-06-29

## Scope

This slice moved non-streaming chat completion response schemas from
`crates/ferrite-server/src/openai/schema/chat.rs` into the focused
`crates/ferrite-server/src/openai/schema/chat_response.rs` module.

The request parser, chat message validation, and chat role handling remain in
`schema/chat.rs`. The public schema re-export is preserved through
`crates/ferrite-server/src/openai/schema.rs`, so route code continues to import
`ChatCompletionResponse` from the schema facade.

This is a schema-organization slice only. It does not change request parsing,
response JSON shape, response IDs, finish reasons, usage accounting, routing,
or inference execution.

## Verification

Before the move:

```sh
cargo test -p ferrite-server --lib openai::schema::chat::tests::chat_completion_response_ids_are_unique_within_the_same_second -- --nocapture
cargo test -p ferrite-server --lib openai::response_shape_tests:: -- --nocapture
cargo test -p ferrite-server --lib openai::routes_tests:: -- --nocapture
```

Observed results:

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 254 filtered out; finished in 0.00s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 251 filtered out; finished in 0.02s
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 243 filtered out; finished in 0.03s
```

After the move:

```sh
cargo test -p ferrite-server --lib openai::schema::chat_response::tests::chat_completion_response_ids_are_unique_within_the_same_second -- --nocapture
cargo test -p ferrite-server --lib openai::schema::chat::tests:: -- --nocapture
cargo test -p ferrite-server --lib openai::response_shape_tests:: -- --nocapture
cargo test -p ferrite-server --lib openai::routes_tests:: -- --nocapture
```

Observed results:

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 254 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 252 filtered out; finished in 0.00s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 251 filtered out; finished in 0.01s
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 243 filtered out; finished in 0.03s
```

`schema/chat.rs` now contains 406 lines, while `schema/chat_response.rs`
contains 115 lines.
