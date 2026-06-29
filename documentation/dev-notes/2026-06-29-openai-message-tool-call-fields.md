# OpenAI Message Tool-Call Fields

Date: 2026-06-29

## Slice

Ferrite's OpenAI-compatible chat completion endpoint now rejects active
message-level tool/function call metadata:

- `messages[].tool_calls` with one or more tool calls
- `messages[].function_call` with an active function-call object

These fields are not harmless transcript text. Accepting them without tool
execution, tool-call replay, or structured function-call semantics would make a
request appear supported while Ferrite only used the message `content`.

Neutral forms remain compatible with the existing no-tool policy:
`tool_calls: []`, missing fields, null fields, and `function_call: "none"` are
treated as no-op metadata.

Reference:

- <https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create/>

## Red

The focused route tests first proved that active message-level tool/function
fields were silently ignored and the request reached generation:

```sh
cargo test -p ferrite-server message_tool_call_fields -- --nocapture
cargo test -p ferrite-server message_function_call_fields -- --nocapture
```

Both failed before implementation with `503` instead of the desired
validation-layer `400`.

## Green

Changes:

- Added explicit `tool_calls` and `function_call` request fields to
  `ChatMessage`.
- Added message-level unsupported-field detection that reports
  `messages.tool_calls` and `messages.function_call`.
- Preserved existing neutral no-tool/no-function handling by reusing the
  established schema helpers.

Verification:

```sh
cargo test -p ferrite-server message_tool_call_fields -- --nocapture
cargo test -p ferrite-server message_function_call_fields -- --nocapture
```

Both focused tests passed after implementation.

## Boundary

This slice does not implement tool execution, tool-result message replay,
function-call argument generation, or hosted tools. It only prevents active
message-level tool/function call metadata from being silently dropped.
