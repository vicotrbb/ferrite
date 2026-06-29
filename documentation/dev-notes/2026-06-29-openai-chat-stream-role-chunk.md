# OpenAI Chat Stream Role Chunk

Date: 2026-06-29

## Slice

Ferrite's OpenAI-compatible streaming chat completion response now starts with
an assistant role chunk before emitting content chunks:

- initial chunk: `choices[].delta.role: "assistant"` with empty content;
- content chunks: `choices[].delta.content`;
- stop chunk: empty `delta` with `finish_reason: "stop"`.

OpenAI's Chat Completions streaming examples show an initial assistant role
chunk before content deltas. This makes Ferrite friendlier to clients that use
the first stream event to initialize assistant message state.

Source reference:

- https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create

## Implementation

- Extended `ChatCompletionStreamContext` with a role chunk constructor.
- Updated non-live `ChatCompletionStreamChunk::from_generation` to include the
  role chunk before token chunks.
- Updated the live streaming route path to send chat-only initial chunks before
  generation starts; legacy completion streaming still sends no initial chunks.
- Extended `response_shape_tests` to parse SSE JSON events and assert the
  initial assistant role chunk, token chunk, stop chunk, and neutral
  `logprobs: null` shape.

## Red Test

```sh
cargo test -p ferrite-server chat_stream_endpoint_returns_openai_choice_shape -- --nocapture
```

Failed before implementation because the first streamed event was a content
chunk with no assistant role:

```text
{"choices":[{"delta":{"content":"winner"},"finish_reason":null,"index":0,"logprobs":null}],...}
```

## Validation

```sh
cargo fmt --all -- --check
cargo test -p ferrite-server response_shape -- --nocapture
cargo test -p ferrite-server -- --nocapture
cargo clippy -p ferrite-server --all-targets -- -D warnings
git diff --check
```

All commands passed after implementation.

## Limits

This slice does not implement tool-call streaming, role changes after the
initial assistant role chunk, streaming logprob payloads, or system fingerprint
metadata. It only aligns the local text chat stream envelope with the documented
OpenAI role-delta shape.
