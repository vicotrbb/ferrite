# OpenAI Unsupported Content Parts

Date: 2026-06-29

## Slice

Ferrite's OpenAI-compatible chat completion endpoint now deserializes
unsupported message content part tags far enough to report them through the
normal unsupported-field path. Image and audio input content parts now return
OpenAI-shaped `messages.content` errors instead of generic JSON body
deserialization failures.

OpenAI documents user message content parts for text, image, audio, and file
inputs. Ferrite's current local inference path is text-only, so non-text input
parts remain unsupported.

Source reference:

- https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create

## Implementation

- Added an unsupported-content marker to `ChatContent`.
- Kept text content parts and assistant refusal content parts on their existing
  local transcript paths.
- Updated chat message validation to reject any unsupported content part as
  `messages.content`.

## Red Tests

```sh
cargo test -p ferrite-server image_content_parts -- --nocapture
cargo test -p ferrite-server audio_content_parts -- --nocapture
```

Both tests failed before implementation because the request body failed
deserialization before Ferrite's unsupported-field validation ran.

## Validation

```sh
cargo test -p ferrite-server image_content_parts -- --nocapture
cargo test -p ferrite-server audio_content_parts -- --nocapture
cargo test -p ferrite-server chat_content -- --nocapture
```

All three commands passed after implementation.

## Limits

This slice does not implement multimodal inference, image input, audio input,
file input, hosted file retrieval, or any non-text prompt rendering.
