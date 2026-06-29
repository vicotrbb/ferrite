# OpenAI Malformed Content Parts

Date: 2026-06-29

## Slice

Ferrite's OpenAI-compatible chat completion endpoint now parses malformed
known content-part tags far enough to return the normal unsupported-field
error path. Text content parts with missing or non-string `text` values now
return OpenAI-shaped `messages.content` errors instead of generic JSON body
deserialization failures.

OpenAI documents text content parts as `{ "type": "text", "text": string }`.
Ferrite accepts only valid text strings for local prompt rendering.

Source reference:

- https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create

## Implementation

- Replaced the derived content-part enum deserializer with a small custom
  parser.
- Valid text parts and valid assistant refusal parts still become local
  transcript text.
- Malformed known tags and unsupported tags become unsupported markers for
  chat request validation.

## Red Tests

```sh
cargo test -p ferrite-server malformed_text_content_parts -- --nocapture
cargo test -p ferrite-server non_string_text_content_parts -- --nocapture
```

Both tests failed before implementation because the request body failed
deserialization before Ferrite's unsupported-field validation ran.

## Validation

```sh
cargo test -p ferrite-server malformed_text_content_parts -- --nocapture
cargo test -p ferrite-server non_string_text_content_parts -- --nocapture
cargo test -p ferrite-server chat_content -- --nocapture
```

All three commands passed after implementation.

## Limits

This slice does not implement multimodal input, file input, audio input, image
input, or recovery from malformed content as prompt text.
