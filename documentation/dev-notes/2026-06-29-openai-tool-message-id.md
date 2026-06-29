# OpenAI Tool Message ID

Date: 2026-06-29

## Slice

Ferrite's OpenAI-compatible chat completion endpoint now rejects `tool`
messages that omit `tool_call_id`.

OpenAI documents tool messages as `{ content, role, tool_call_id }`. Ferrite
still treats tool messages as local transcript text in this slice, but accepting
a tool message without its tool-call correlation ID would make malformed tool
transcript data look supported.

Reference:

- <https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create/>

## Red

The focused route test first sent a tool message without `tool_call_id`:

```sh
cargo test -p ferrite-server tool_message_without_tool_call_id -- --nocapture
```

It failed before implementation with `503` instead of the desired
validation-layer `400`, proving the request reached generation.

## Green

Changes:

- Added role-aware `ChatMessage` validation for `role: "tool"`.
- `tool_call_id` remains optional for non-tool message roles.
- Missing or malformed tool-message IDs are reported as
  `messages.tool_call_id`.

Verification:

```sh
cargo test -p ferrite-server tool_message_without_tool_call_id -- --nocapture
cargo test -p ferrite-server malformed_message_tool_call_id -- --nocapture
cargo test -p ferrite-server malformed_message_name -- --nocapture
```

All focused tests passed after implementation.

## Boundary

This slice does not implement tool-call matching, tool execution, or
tool-result replay semantics. It only enforces the documented correlation ID
before Ferrite treats tool messages as plain local transcript context.
