# OpenAI Chat Response Shape

Date: 2026-06-29

## Slice

Ferrite's OpenAI-compatible non-streaming chat completion response now includes
the documented assistant-message compatibility fields:

- `choices[].logprobs: null`
- `choices[].message.refusal: null`
- `choices[].message.annotations: []`

OpenAI's Chat Completions API documents `choices[].logprobs` on chat
completion choices and `refusal` plus `annotations` on generated chat
completion messages. Ferrite emits neutral values because the current local
inference path does not implement logprob reporting, refusal generation, or web
search annotations.

Source reference:

- https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create

## Implementation

- Added `crates/ferrite-server/src/openai/response_shape_tests.rs` so response
  shape checks do not further grow the large route test module.
- Updated `ChatCompletionChoice` to serialize `logprobs: null`.
- Updated `ChatCompletionMessage` to serialize `refusal: null` and
  `annotations: []`.

## Red Test

```sh
cargo test -p ferrite-server chat_endpoint_returns_openai_message_shape -- --nocapture
```

Failed before implementation with a response body that omitted the new fields:

```text
{"choices":[{"finish_reason":"stop","index":0,"message":{"content":"winner","role":"assistant"}}],...}
```

## Validation

```sh
cargo fmt --all -- --check
cargo test -p ferrite-server chat_endpoint_returns_openai_message_shape -- --nocapture
cargo test -p ferrite-server -- --nocapture
cargo clippy -p ferrite-server --all-targets -- -D warnings
git diff --check
```

All commands passed after implementation.

## Limits

This slice does not implement token log probability reporting, refusal
classification, web search annotations, tool-call annotations, moderation
output, or multimodal response metadata. It only makes Ferrite's existing
non-streaming text chat response closer to the documented OpenAI response
shape.
