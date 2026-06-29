# OpenAI Assistant Tool-Call Content Optional

Date: 2026-06-29

## Slice

Ferrite's OpenAI-compatible chat completion endpoint now parses assistant
messages that omit `content` when they include `tool_calls` or deprecated
`function_call` metadata. Ferrite still does not implement tool calling; the
change only moves these OpenAI-valid transcript shapes from generic JSON
deserialization failure to Ferrite's explicit unsupported-field error path.

OpenAI documents assistant `content` as optional and required unless
`tool_calls` or `function_call` is specified.

Source reference:

- https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create

## Implementation

- Changed chat message content parsing to store optional `ChatContent`.
- Kept local prompt rendering unchanged for supported messages.
- Added role-aware content validation: plain messages without content are
  rejected as `messages.content`, while assistant messages with tool metadata
  are allowed to deserialize and then rejected as `messages.tool_calls` or
  `messages.function_call`.

## Red Tests

```sh
cargo test -p ferrite-server message_tool_call_fields_without_content -- --nocapture
cargo test -p ferrite-server message_function_call_fields_without_content -- --nocapture
cargo test -p ferrite-server message_without_content -- --nocapture
```

All three tests failed before implementation because the JSON body could not be
deserialized into the request type.

## Validation

```sh
cargo test -p ferrite-server message_tool_call_fields_without_content -- --nocapture
cargo test -p ferrite-server message_function_call_fields_without_content -- --nocapture
cargo test -p ferrite-server message_without_content -- --nocapture
```

All three commands passed after implementation.

## Limits

This slice does not implement tool calling, function calling, tool execution,
assistant tool-call response generation, or local handling of tool results.
