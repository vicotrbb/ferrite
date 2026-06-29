# OpenAI System Fingerprint

Ferrite now includes `system_fingerprint: null` on OpenAI-compatible chat
completion responses and chat completion stream chunks.

## Why

OpenAI's Chat Completions API documents `system_fingerprint` as a deprecated
optional response field representing the backend configuration used by the
model. Ferrite does not expose an equivalent OpenAI backend fingerprint, so the
local server reports the field as JSON `null` rather than inventing a value.

Reference:
https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create

## Changes

- Added nullable `system_fingerprint` to `ChatCompletionResponse`.
- Added nullable `system_fingerprint` to `ChatCompletionStreamChunk`.
- Extended response-shape tests to cover non-streaming chat responses and
  role, token, and stop stream chunks.

## TDD Evidence

Red test:

```bash
cargo test -p ferrite-server response_shape -- --nocapture
```

Expected failures before implementation:

```text
openai::response_shape_tests::chat_endpoint_returns_openai_message_shape ... FAILED
openai::response_shape_tests::chat_stream_endpoint_returns_openai_choice_shape ... FAILED
```

Focused green check:

```bash
cargo test -p ferrite-server response_shape -- --nocapture
```
