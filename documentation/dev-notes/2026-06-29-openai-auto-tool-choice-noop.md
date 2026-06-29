# OpenAI Auto Tool Choice No-Op

Date: 2026-06-29

## Scope

Accept `tool_choice: "auto"` and deprecated `function_call: "auto"` on
OpenAI-compatible chat completion requests only when the request does not
provide active tools or functions.

This is a compatibility slice for common OpenAI client defaults. It does not
implement tool calling, function calling, forced tool selection, generated
tool-call messages, or hosted tool execution.

Source reference:

- https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create/

## Rationale

The OpenAI Chat Completions reference treats `tool_choice` and deprecated
`function_call` as controls over whether the model may call tools/functions.
With no tools or functions available, `auto` cannot select a tool call and is
equivalent to local text generation. Rejecting it creates unnecessary friction
for clients that send default `auto` values even when they are not configuring
tools.

Ferrite still rejects `auto` when active tools/functions are present, because
silently dropping real tool-calling intent would be misleading.

## Change

- Added fixture chat route coverage for `tool_choice: "auto"` without tools.
- Added fixture chat route coverage for `function_call: "auto"` without
  functions.
- Kept message-level `function_call` validation strict by separating request
  option handling from assistant transcript metadata handling.
- Preserved explicit rejection for active tool/function request fields.

## Red Tests

```sh
cargo test -p ferrite-server --lib openai::chat_option_tests -- --nocapture
```

Observed result before implementation:

- `chat_endpoint_accepts_auto_tool_choice_without_tools` failed with HTTP `400`
  and `error.param = "tool_choice"`.
- `chat_endpoint_accepts_auto_function_call_without_functions` failed with HTTP
  `400` and `error.param = "function_call"`.
- The rest of the suite passed.

## Validation

Post-implementation validation:

```sh
cargo test -p ferrite-server --lib openai::chat_option_tests -- --nocapture
cargo test -p ferrite-server --lib openai::unsupported_tests -- --nocapture
cargo test -p ferrite-server --lib openai::chat_message_tool_tests -- --nocapture
cargo test -p ferrite-server --lib function_options -- --nocapture
cargo test -p ferrite-server --lib tool_options -- --nocapture
cargo fmt --all -- --check
git diff --check
CARGO_BUILD_JOBS=2 cargo clippy -p ferrite-server --all-targets -- -D warnings
```

Observed result:

- `openai::chat_option_tests`: 19 passed.
- `openai::unsupported_tests`: 9 passed.
- `openai::chat_message_tool_tests`: 8 passed.
- `function_options` filter: 4 passed.
- `tool_options` filter: 4 passed.
- Formatting check passed.
- Whitespace check passed.
- `ferrite-server` clippy passed with warnings denied.

## Limits

This slice does not add tool execution support and does not rerun ignored
real-model GGUF HTTP suites.
