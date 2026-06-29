# OpenAI Stop Sequences

Date: 2026-06-29

## Summary

Ferrite's OpenAI-compatible chat and legacy completion endpoints now accept
supported `stop` sequence requests and apply them at the server generation
boundary:

- `stop: "..."`;
- `stop: ["...", "..."]` with up to four non-empty strings; and
- `stop: []` as a no-op.

This is server-side text stopping for the local OpenAI-compatible API. It does
not add stochastic sampling, multi-choice generation, tokenizer-level stop
criteria, or inference-core OpenAI request types.

## Implementation Notes

- Added a focused `openai::stop_sequences_tests` module instead of growing the
  already broad route test file.
- Updated stop-sequence schema validation to accept the supported OpenAI local
  forms and reject malformed stop values.
- Threaded parsed stop strings as `Vec<String>` through server routes into
  `openai::generation`.
- Added server-side stop filtering for non-streaming and streaming responses:
  generated text is trimmed before the first stop sequence, and SSE token chunks
  suppress text after the first stop match while still emitting the terminal
  OpenAI `finish_reason: "stop"` chunk and `[DONE]`.

The inference runtime still generates tokens with Ferrite-owned model/session
logic. This slice keeps stop-sequence handling in the HTTP/server layer.

## Verification

Red check:

```sh
cargo test -p ferrite-server openai::stop_sequences_tests -- --nocapture
```

Observed failures before implementation:

- `completions_endpoint_applies_string_stop_sequence` returned `400` with
  `unsupported completion field(s): stop`.
- `chat_endpoint_applies_string_stop_sequence` returned `400` with
  `unsupported chat completion field(s): stop`.
- `completions_stream_endpoint_applies_string_stop_sequence` returned `400`
  with `unsupported completion field(s): stop`.
- `chat_stream_endpoint_applies_string_stop_sequence` returned `400` with
  `unsupported chat completion field(s): stop`.

Focused green check:

```sh
cargo test -p ferrite-server openai::stop_sequences_tests -- --nocapture
```

Observed result:

- `openai::stop_sequences_tests`: 4 passed.

The fixture model generates `winner`; all four endpoint shapes now return or
stream visible text `win` for `stop: "ner"` while preserving usage accounting
for the generated token.

## Real Tier 1 HTTP Regression

Follow-up non-streaming command:

```sh
cargo test -p ferrite-server --test openai_real_tier1_http live_http_server_applies_stop_sequences_with_real_tier1_model -- --ignored --nocapture
```

Observed result:

- `live_http_server_applies_stop_sequences_with_real_tier1_model`: 1 passed;
  0 failed; 6 filtered out; finished in 54.21s.

This explicit opt-in run loaded the local
`target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf` Tier 1 artifact through the
OpenAI-compatible HTTP server. It verified that `stop: "\n"` trims the known
legacy completion token for `hello world` to empty visible text, and that
`stop: "你"` trims the known chat completion token to empty visible content,
while both responses still report one generated completion token.

Follow-up streaming command:

```sh
cargo test -p ferrite-server --test openai_real_tier1_http live_http_server_streams_stop_sequences_with_real_tier1_model -- --ignored --nocapture
```

Observed result:

- `live_http_server_streams_stop_sequences_with_real_tier1_model`: 1 passed;
  0 failed; 7 filtered out; finished in 35.81s.

This explicit opt-in run used the same local Qwen2.5-0.5B Q4_K_M artifact and
verified the SSE paths. `stop: "\n"` suppresses the known legacy completion
token from visible text chunks, and `stop: "你"` suppresses the known chat
completion token from visible content chunks. Both streams still emit exactly
one terminal `finish_reason: "stop"` event and `data: [DONE]`.
