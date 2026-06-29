# OpenAI Stream Service Tier

Date: 2026-06-29

## Scope

Ferrite's OpenAI-compatible streaming chat completion endpoint now includes
`service_tier: "default"` on streamed chunks when the request sets a supported
local service tier.

## Context

The Chat Completions API says the response body includes the actual
`service_tier` value when the request sets `service_tier`. The OpenAI Python
SDK's `ChatCompletionChunk` type also includes an optional `service_tier` field
on streamed chat completion chunks.

Sources:

- https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create
- https://github.com/openai/openai-python/blob/main/src/openai/types/chat/chat_completion_chunk.py

## Red

Added a streaming route test for:

```json
{
  "model": "fixture-model",
  "messages": [{ "role": "user", "content": "hello" }],
  "max_completion_tokens": 1,
  "stream": true,
  "service_tier": "auto"
}
```

Initial focused run:

```text
cargo test -p ferrite-server chat_stream_endpoint_includes_service_tier -- --nocapture
```

The new test failed because the SSE chunks contained `id`, `object`, `created`,
`model`, `system_fingerprint`, `choices`, and optional usage fields, but no
`service_tier`.

## Implementation

- Added an optional `service_tier` field to chat stream chunks.
- Added `ChatCompletionStreamContext::with_service_tier`.
- Passed `ChatCompletionRequest::response_service_tier()` into the streaming
  chat response path.
- Kept the value out of inference core types; this remains an HTTP response
  shape concern.

## Green

Focused verification after implementation:

```text
cargo test -p ferrite-server chat_stream_endpoint_includes_service_tier -- --nocapture
cargo test -p ferrite-server service_tier -- --nocapture
```

Results:

```text
chat_stream_endpoint_includes_service_tier ... ok
openai::service_tier_tests ... 3 passed; 0 failed
openai::schema::service_tier ... 3 passed; 0 failed
```

## Limits

Ferrite still only accepts `service_tier: "auto"` and
`service_tier: "default"` as local no-ops. It does not implement Flex,
Priority, Scale, paid-tier routing, or any scheduling behavior tied to service
tier.
