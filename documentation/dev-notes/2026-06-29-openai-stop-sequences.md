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
