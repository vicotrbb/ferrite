# OpenAI No-Function Options

Date: 2026-06-29

## Slice

Ferrite's OpenAI-compatible chat endpoint now accepts the deprecated
function-calling fields when they explicitly request no function behavior:
`functions: []` and `function_call: "none"`. The official Chat Completions API
documents these fields as deprecated in favor of `tools` and `tool_choice`.
Ferrite still rejects real function-calling requests.

Source reference:

- https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create

## Implementation

- Added `crates/ferrite-server/src/openai/schema/function_options.rs` to keep
  deprecated function option validation separate from chat request logic.
- Updated chat unsupported-field detection to accept missing `functions`,
  empty `functions`, missing `function_call`, or `function_call: "none"`.
- Added a fixture-backed route test for explicit no-function options.
- Added a route-level rejection regression for real deprecated function fields.

## Red Test

```sh
cargo test -p ferrite-server chat_endpoint_accepts_explicit_no_function_options -- --nocapture
```

Failed before implementation with:

```text
unsupported chat completion field(s): function_call, functions
```

## Validation

```sh
cargo test -p ferrite-server chat_endpoint_accepts_explicit_no_function_options -- --nocapture
cargo test -p ferrite-server openai::schema::function_options -- --nocapture
cargo test -p ferrite-server chat_endpoint_rejects_function_fields -- --nocapture
```

All commands passed after implementation.

## Limits

This slice does not implement deprecated OpenAI function calling, function
selection, function-call response messages, JSON-argument generation, or any
tool/function execution behavior. `function_call: "auto"`, forced function
calls, and non-empty `functions` remain unsupported.
