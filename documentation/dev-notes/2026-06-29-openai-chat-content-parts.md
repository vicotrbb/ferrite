# OpenAI Chat Text Content Parts

Date: 2026-06-29

## Summary

Ferrite's OpenAI-compatible chat endpoint now accepts text-only message content
parts in addition to plain string message content.

The supported local-serving shape is:

```json
{"role":"user","content":[{"type":"text","text":"hello"}]}
```

Ferrite concatenates text parts before rendering the existing local prompt.
Non-text content parts such as images, audio, or files remain unsupported and
fail request deserialization as an OpenAI-shaped `invalid_request_error`.

This matches the local text-generation scope in ADR 0008 while accepting a
current OpenAI Chat Completions request shape. The official OpenAI API
reference describes chat message `content` as either a string or an array of
typed content parts, with text parts represented as `{ "type": "text",
"text": "..." }`.

Reference:

- <https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create/>

## Implementation Notes

- Added `openai::schema::chat_content` as a focused parser for chat message
  content.
- Kept the route and prompt renderer on plain text via `ChatMessage::content()`.
- Added a fixture-backed route test for array content parts.
- Added parser tests for string content, text content parts, and non-text part
  rejection.

## Verification

Red test:

```sh
cargo test -p ferrite-server openai::routes_tests::chat_endpoint_accepts_text_content_parts -- --nocapture
```

Initial result before implementation:

- The request returned `400` with `Failed to deserialize the JSON body into the
  target type`.

Focused final checks:

```sh
cargo test -p ferrite-server openai::routes_tests::chat_endpoint_accepts_text_content_parts -- --nocapture
cargo test -p ferrite-server openai::schema::chat_content -- --nocapture
```

Observed result:

- `chat_endpoint_accepts_text_content_parts`: 1 passed.
- `openai::schema::chat_content`: 3 passed.
