# OpenAI Stream Obfuscation Enabled

## Scope

Ferrite's OpenAI-compatible streaming endpoints now accept
`stream_options.include_obfuscation: true`.

When enabled, Ferrite includes an opaque `obfuscation` string on streaming
chunks for both:

- `POST /v1/completions` with `stream: true`
- `POST /v1/chat/completions` with `stream: true`

Source reference:

- <https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create/>

## TDD Evidence

RED command:

```sh
cargo test -p ferrite-server --lib completion_stream_endpoint_emits_obfuscation_when_requested -- --nocapture
```

Observed failure:

```text
assertion `left == right` failed
  left: 400
 right: 200
test openai::stream_options_tests::completion_stream_endpoint_emits_obfuscation_when_requested ... FAILED
```

GREEN commands:

```sh
cargo test -p ferrite-server --lib completion_stream_endpoint_emits_obfuscation_when_requested -- --nocapture
cargo test -p ferrite-server --lib chat_stream_endpoint_emits_obfuscation_when_requested -- --nocapture
```

Observed result:

```text
test openai::stream_options_tests::completion_stream_endpoint_emits_obfuscation_when_requested ... ok
test openai::stream_options_tests::chat_stream_endpoint_emits_obfuscation_when_requested ... ok
```

## Boundary

The obfuscation string is local opaque padding for OpenAI client compatibility.
It is not a security primitive and does not change generated token content,
usage accounting, stop handling, model execution, or sampling behavior.
