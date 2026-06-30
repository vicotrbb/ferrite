# OpenAI async client obfuscation proof

## Context

Ferrite now accepts `stream_options.include_obfuscation` and emits an
`obfuscation` field on streaming chat and legacy completion chunks when the
option is enabled. The raw HTTP tests prove the field is present, but the
OpenAI-compatible server milestone also requires proving common OpenAI clients
can use Ferrite with a local base URL.

## Slice

Add `async-openai` integration coverage for streaming chat completions and
legacy completions with `include_obfuscation: true`.

`async-openai` 0.41.1 exposes `include_obfuscation` on
`ChatCompletionStreamOptions`, but its chat and legacy completion stream
response structs do not expose an `obfuscation` response field. The client uses
Serde's default unknown-field behavior, so the compatibility proof is that the
stream request completes successfully and still yields the expected content and
usage when Ferrite emits the extra field.

## Validation

Executed:

- `cargo test -p ferrite-server --test openai_client_chat async_openai_client_streams_chat_completion_with_obfuscation -- --nocapture`
- `cargo test -p ferrite-server --test openai_client_completions async_openai_client_streams_legacy_completion_with_obfuscation -- --nocapture`

Both focused tests passed. No production-code change was required for this
slice.
