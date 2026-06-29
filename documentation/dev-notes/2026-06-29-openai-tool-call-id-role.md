# OpenAI Tool Call ID Role Boundary

Date: 2026-06-29

## Slice

Ferrite's OpenAI-compatible chat completion endpoint now rejects
`tool_call_id` on non-tool messages.

The OpenAI Chat Completions reference defines `tool_call_id` as part of the
tool-message shape. Ferrite still treats tool messages as local transcript text
in this slice, but accepting the correlation ID on unrelated message roles
would make malformed transcript data look supported.

Reference:

- <https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create/>

## Red

The focused route test first sent a user message with `tool_call_id`:

```sh
cargo test -p ferrite-server tool_call_id_on_non_tool_message -- --nocapture
```

It failed before implementation with `503` instead of the desired
validation-layer `400`, proving the request reached generation.

## Green

Changes:

- Added role-aware `ChatMessage` validation for `tool_call_id`.
- `tool_call_id` is accepted only on `role: "tool"` messages.
- Tool messages still require a string `tool_call_id`.
- Non-tool use and malformed values are reported as `messages.tool_call_id`.

Verification:

```sh
cargo test -p ferrite-server tool_call_id_on_non_tool_message -- --nocapture
cargo test -p ferrite-server tool_message_without_tool_call_id -- --nocapture
cargo test -p ferrite-server malformed_message_tool_call_id -- --nocapture
```

All focused tests passed after implementation.

## Boundary

This slice does not implement tool-call matching, tool execution, or
tool-result replay semantics. It only keeps the documented tool-message
correlation field role-scoped before Ferrite treats accepted messages as local
transcript context.
