# OpenAI Message Metadata Types

Date: 2026-06-29

## Slice

Ferrite's OpenAI-compatible chat completion endpoint now validates documented
message-level metadata types:

- `messages[].name` must be a string when present.
- `messages[].tool_call_id` must be a string when present.

These fields remain local no-op metadata in this slice. The validation prevents
malformed metadata objects or numbers from being silently ignored while Ferrite
uses only `role` and `content` for prompt rendering.

Reference:

- <https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create/>

## Red

The focused route tests first sent malformed metadata values:

```sh
cargo test -p ferrite-server malformed_message_name -- --nocapture
cargo test -p ferrite-server malformed_message_tool_call_id -- --nocapture
```

Both failed before implementation with `503` instead of the desired
validation-layer `400`, proving the requests reached generation.

## Green

Changes:

- Added `openai::schema::message_metadata` as a focused optional-string
  validator for message metadata.
- Updated `ChatMessage::unsupported_fields()` to report malformed values as
  `messages.name` and `messages.tool_call_id`.
- Added route tests covering malformed `name` and `tool_call_id` values.

Verification:

```sh
cargo test -p ferrite-server malformed_message_name -- --nocapture
cargo test -p ferrite-server malformed_message_tool_call_id -- --nocapture
cargo test -p ferrite-server openai::schema::message_metadata -- --nocapture
```

All focused tests passed after implementation.

## Boundary

This slice does not implement tool-result replay, tool-call matching, or
message-name prompt rendering. It only validates documented metadata types so
malformed request data is not accepted as if it had local meaning.
