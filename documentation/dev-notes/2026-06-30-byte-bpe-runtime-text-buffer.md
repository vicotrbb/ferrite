# 2026-06-30 Byte-BPE Runtime Text Buffer

## Summary

Ferrite now buffers generated token IDs until their decoded text is complete
UTF-8 before invoking runtime token text callbacks.

The failure was found during a real OpenAI-compatible HTTP chat probe against
Qwen2.5-1.5B-Instruct Q8_0. `POST /v1/chat/completions` returned HTTP `500`
because the runtime decoded each generated token independently, but byte-level
BPE tokens can represent incomplete UTF-8 bytes until adjacent tokens arrive.

## Changes

- Added `TokenizerError::is_incomplete_utf8`.
- Added `GgufTokenizer::decode_if_complete`.
- Marked incomplete BPE UTF-8 decode failures as recoverable tokenizer errors.
- Added a server-side token text buffer so callbacks receive decoded text
  chunks, while `completion_tokens` continues to count generated token IDs.

## Verification

Red tests before implementation:

```text
cargo test -p ferrite-model --test tokenizer_metadata bpe_reports_incomplete_utf8_for_partial_byte_token
error[E0599]: no method named `is_incomplete_utf8` found for struct `TokenizerError`
error[E0599]: no method named `decode_if_complete` found for struct `GgufTokenizer`

cargo test -p ferrite-server token_text_buffer_waits_for_decodable_utf8_sequence
error[E0433]: cannot find type `TokenTextBuffer` in this scope
```

Focused green checks after implementation:

```text
cargo test -p ferrite-model --test tokenizer_metadata
test result: ok. 7 passed; 0 failed

cargo test -p ferrite-server --lib runtime::tests
test result: ok. 2 passed; 0 failed

cargo fmt -- --check

cargo clippy -p ferrite-model -p ferrite-server --all-targets -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s)
```

Release build:

```text
cargo build --release -p ferrite-server
Finished `release` profile [optimized] target(s) in 6.16s
```

Real model HTTP repro after the fix:

```text
GET /health
{"status":"ok","ready":true,"model":"qwen2.5-1.5b-q8_0-chat32"}

POST /v1/chat/completions
http_code=200
time_total=3.983436
object=chat.completion
model=qwen2.5-1.5b-q8_0-chat32
finish_reason=length
role=assistant
content_length=68
prompt_tokens=8
completion_tokens=32
total_tokens=40
```

A second identical request also returned HTTP `200` with
`object=chat.completion`, `finish_reason=length`, and `completion_tokens=32`.

## Remaining Limits

This proves the specific incomplete UTF-8 token decode failure is fixed for the
tested Qwen2.5-1.5B Q8_0 chat shape. It does not prove SSE streaming text
framing, concurrent request behavior, long-running leak freedom, or complete
Tier 1 memory posture.
