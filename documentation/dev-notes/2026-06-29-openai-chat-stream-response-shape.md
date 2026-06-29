# OpenAI Chat Stream Response Shape

Date: 2026-06-29

## Slice

Ferrite's OpenAI-compatible streaming chat completion chunks now include
`choices[].logprobs: null` on token and stop chunks. OpenAI's Chat Completions
streaming examples show `logprobs: null` alongside `delta` and `finish_reason`
for streamed choices when log probability reporting is not enabled.

Source reference:

- https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create

## Implementation

- Extended `crates/ferrite-server/src/openai/response_shape_tests.rs` with a
  focused SSE response-shape regression that parses JSON `data:` events.
- Updated `ChatCompletionStreamChoice` to serialize `logprobs: null` for both
  content chunks and the terminal stop chunk.

## Red Test

```sh
cargo test -p ferrite-server chat_stream_endpoint_returns_openai_choice_shape -- --nocapture
```

Failed before implementation with a streamed choice that omitted `logprobs`:

```text
{"choices":[{"delta":{"content":"winner"},"finish_reason":null,"index":0}],...}
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

This slice does not implement streaming token log probability reporting,
`logprobs: true`, `top_logprobs`, token byte spans, or top-token alternatives.
Ferrite still rejects behavior-changing logprob requests and emits a neutral
`null` field for compatibility only.
