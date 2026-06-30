# OpenAI Streaming Completion Echo

## Scope

Ferrite's OpenAI-compatible legacy completions endpoint now supports
`"echo": true` on streaming requests.

For `POST /v1/completions` with `stream: true`, Ferrite emits the original
prompt as the first `text_completion` SSE chunk before generated token chunks.
Malformed `echo` values remain unsupported.

## TDD Evidence

RED command:

```sh
cargo test -p ferrite-server --lib completions_endpoint_streams_echo_prompt_when_requested -- --nocapture
```

Observed failure:

```text
assertion `left == right` failed: {"error":{"message":"unsupported completion field(s): echo","type":"invalid_request_error","param":"echo","code":null}}
  left: 400
 right: 200
test openai::route_streaming_tests::completions_endpoint_streams_echo_prompt_when_requested ... FAILED
```

GREEN command:

```sh
cargo test -p ferrite-server --lib completions_endpoint_streams_echo_prompt_when_requested -- --nocapture
```

Observed result:

```text
test openai::route_streaming_tests::completions_endpoint_streams_echo_prompt_when_requested ... ok
```

## Boundary

This only echoes the original prompt text for legacy completion streams. It
does not add logprobs for prompt tokens, chat completion echo behavior, or new
sampling behavior.
