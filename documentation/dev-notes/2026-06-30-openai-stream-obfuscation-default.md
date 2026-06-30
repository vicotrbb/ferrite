# OpenAI stream obfuscation default

## Context

OpenAI's Chat Completions API reference says `stream_options.include_obfuscation`
adds an `obfuscation` field to streaming delta events, that these fields are
included by default, and that clients can set `include_obfuscation` to `false`
to optimize for bandwidth.

Reference:

- <https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create/>

Ferrite previously emitted `obfuscation` only when clients explicitly sent
`"include_obfuscation": true`. That made the opt-in behavior work, but it did
not match the documented default.

## Slice

Default streaming obfuscation to enabled for both OpenAI-compatible endpoint
families:

- `POST /v1/completions`
- `POST /v1/chat/completions`

Explicit `stream_options.include_obfuscation: false` still disables the field.

## RED

Before the implementation change:

- `cargo test -p ferrite-server --lib openai::stream_options_tests -- --nocapture`

failed the new default-obfuscation endpoint tests:

- `completion_stream_endpoint_emits_obfuscation_by_default`
- `chat_stream_endpoint_emits_obfuscation_by_default`

The failure bodies contained streamed completion/chat chunks and `data: [DONE]`,
but no `obfuscation` field.

## GREEN

After the implementation change:

- `cargo test -p ferrite-server --lib openai::stream_options_tests -- --nocapture`
- `cargo test -p ferrite-server --lib openai::schema::stream_options -- --nocapture`
- `cargo test -p ferrite-server --test openai_client_chat -- --nocapture`
- `cargo test -p ferrite-server --test openai_client_completions -- --nocapture`
- `cargo fmt --all -- --check`
- `CARGO_BUILD_JOBS=2 cargo clippy -p ferrite-server --all-targets -- -D warnings`

All commands passed.
