# OpenAI Completion Response Shape

Ferrite now includes nullable OpenAI-compatible response-shape fields on
legacy `/v1/completions` responses and stream chunks.

## Why

OpenAI's legacy Completions API response examples include `choices[].logprobs`
and `system_fingerprint`. Ferrite does not currently compute token logprobs or
expose an OpenAI backend fingerprint, so both fields are serialized as JSON
`null`.

Reference:
https://developers.openai.com/api/reference/resources/completions/methods/create

## Changes

- Added `system_fingerprint: null` to `CompletionResponse`.
- Added `choices[].logprobs: null` to non-streaming completion choices.
- Added `system_fingerprint: null` to `CompletionStreamChunk`.
- Added `choices[].logprobs: null` to streamed completion token and stop
  choices.
- Added focused response-shape tests for non-streaming and streaming
  `/v1/completions`.

## TDD Evidence

Red test:

```bash
cargo test -p ferrite-server response_shape -- --nocapture
```

Expected failures before implementation:

```text
openai::response_shape_tests::completions_endpoint_returns_openai_choice_shape ... FAILED
openai::response_shape_tests::completions_stream_endpoint_returns_openai_choice_shape ... FAILED
```

Focused green check:

```bash
cargo test -p ferrite-server response_shape -- --nocapture
```
