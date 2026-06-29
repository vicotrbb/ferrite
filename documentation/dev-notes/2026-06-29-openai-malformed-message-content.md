# OpenAI Malformed Message Content

Date: 2026-06-29

## Scope

Ferrite's OpenAI-compatible chat completion endpoint now returns an explicit
`messages.content` error when a chat message provides malformed content such as
a scalar number.

## Red

Added a route test for:

```json
{
  "model": "fixture-model",
  "messages": [{ "role": "user", "content": 42 }]
}
```

Initial focused run:

```text
cargo test -p ferrite-server openai::unsupported_tests -- --nocapture
```

The new test failed with:

```text
messages must contain at least one item
```

That showed malformed `content` caused the chat message array to deserialize as
empty, hiding the real `messages.content` problem from the client.

## Implementation

- Changed `ChatContent` to parse through `serde_json::Value` instead of an
  untagged enum that can fail the enclosing `ChatMessage`.
- Preserved valid string content, text content parts, and refusal content parts.
- Recorded scalar/object content values as unsupported content so existing
  `ChatMessage::unsupported_fields()` reports `messages.content`.

## Green

Focused verification after implementation:

```text
cargo test -p ferrite-server openai::unsupported_tests -- --nocapture
```

Result:

```text
54 passed; 0 failed; 0 ignored
```

## Limits

This slice does not add support for new OpenAI content forms. It only preserves
malformed content through request parsing long enough to return a precise
OpenAI-shaped `messages.content` validation error.
