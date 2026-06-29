# OpenAI Chat Message Schema Split

Date: 2026-06-29

## Scope

This slice moved chat message and role schemas from
`crates/ferrite-server/src/openai/schema/chat.rs` into the focused
`crates/ferrite-server/src/openai/schema/chat_message.rs` module.

`schema/chat.rs` now focuses on `ChatCompletionRequest`. The sibling
`schema/chat_messages.rs` deserializer imports the message type from the new
module, and `crates/ferrite-server/src/openai/schema.rs` continues to re-export
`ChatMessage` and `ChatRole` through the public schema facade.

This is a schema-organization slice only. It does not change chat request
parsing, message role handling, unsupported-field reporting, prompt rendering,
route behavior, or inference execution.

## Verification

Before the move:

```sh
cargo test -p ferrite-server --lib openai::schema::chat::tests:: -- --nocapture
cargo test -p ferrite-server --lib openai::prompt::tests:: -- --nocapture
cargo test -p ferrite-server --lib openai::chat_message_validation_tests:: -- --nocapture
cargo test -p ferrite-server --lib openai::routes_tests:: -- --nocapture
```

Observed results:

```text
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 252 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 254 filtered out; finished in 0.00s
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 241 filtered out; finished in 0.01s
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 243 filtered out; finished in 0.08s
```

After the move:

```sh
cargo test -p ferrite-server --lib openai::schema::chat_message::tests:: -- --nocapture
cargo test -p ferrite-server --lib openai::chat_message_validation_tests:: -- --nocapture
cargo test -p ferrite-server --lib openai::prompt::tests:: -- --nocapture
cargo test -p ferrite-server --lib openai::routes_tests:: -- --nocapture
cargo fmt --all -- --check
git diff --check
CARGO_BUILD_JOBS=2 cargo clippy -p ferrite-server --all-targets -- -D warnings
```

Observed results:

```text
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 252 filtered out; finished in 0.00s
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 241 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 254 filtered out; finished in 0.00s
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 243 filtered out; finished in 0.03s
cargo fmt --all -- --check exited 0
git diff --check exited 0
cargo clippy exited 0
```

`schema/chat.rs` now contains 224 lines, while `schema/chat_message.rs`
contains 189 lines.
