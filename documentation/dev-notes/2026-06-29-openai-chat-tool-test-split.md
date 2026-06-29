# OpenAI Chat Tool Test Split

Date: 2026-06-29

## Scope

Split chat message tool/function-call validation tests out of the larger
OpenAI unsupported-field test module.

This is a test-organization slice only. It does not change server routing,
request parsing, unsupported-field behavior, OpenAI response shapes, inference,
or documentation visible to end users.

## Rationale

Ferrite's OpenAI compatibility tests have grown as local-client behavior has
expanded. The project goal requires avoiding monster files and keeping Rust
modules focused. `unsupported_tests.rs` mixed top-level chat request validation,
message content validation, sampling validation, metadata validation, and
message tool/function-call validation in one file.

Tool/function-call message validation is a coherent boundary because Ferrite
currently rejects hosted tool-call semantics while accepting explicit no-tool
top-level options elsewhere.

## Change

- Added `crates/ferrite-server/src/openai/chat_message_tool_tests.rs`.
- Moved eight existing tests into the new module:
  - assistant `tool_calls` fields;
  - assistant `function_call` fields;
  - legacy function-message `name` requirements;
  - tool-message `tool_call_id` requirements; and
  - `tool_call_id` rejection on non-tool messages.
- Registered the focused test module from `openai/mod.rs`.
- Left `unsupported_tests.rs` with the remaining non-tool unsupported chat
  request coverage.

## Verification

Focused moved-module check:

```sh
cargo test -p ferrite-server openai::chat_message_tool_tests -- --nocapture
```

Observed result:

- `openai::chat_message_tool_tests`: 8 passed, 0 failed.

Focused remaining-module check:

```sh
cargo test -p ferrite-server openai::unsupported_tests -- --nocapture
```

Observed result:

- `openai::unsupported_tests`: 34 passed, 0 failed.

Line-count check:

```sh
wc -l crates/ferrite-server/src/openai/unsupported_tests.rs \
  crates/ferrite-server/src/openai/chat_message_tool_tests.rs
```

Observed result:

- `unsupported_tests.rs`: 738 lines.
- `chat_message_tool_tests.rs`: 224 lines.

Final gates:

- `cargo fmt --all -- --check`: passed.
- `git diff --check`: passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo test --workspace -- --nocapture`: passed.
- `ferrite-server` library tests: 231 passed, 0 failed.
- Ignored real-model GGUF HTTP suites remained ignored by the default
  workspace run.

## Limits

This split only reduces one OpenAI test-file concern. Other large modules,
including `routes_tests.rs`, still need future focused organization slices.
