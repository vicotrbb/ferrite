# OpenAI return_token_ids Extension

## Goal

Make Ferrite's OpenAI-compatible chat endpoint tolerant of the
`return_token_ids` extension sent by `llama-benchy` without turning unknown
request fields into an untyped escape hatch.

## Context

The first `llama-benchy` compatibility smoke reached Ferrite's
`/v1/chat/completions` endpoint but failed before measurement:

```text
HTTP 400: {"error":{"message":"unsupported chat completion field(s): return_token_ids","type":"invalid_request_error","param":"return_token_ids","code":null}}
```

Inspection of the installed `llama-benchy 0.3.8` client showed that benchmark
generation payloads always include:

```text
"return_token_ids": true
```

Ferrite currently records generated token text pieces for streaming but does
not expose generated token IDs in chat completion responses.

## Change

- Added `return_token_ids` as an explicit `ChatCompletionRequest` field.
- Added `is_optional_bool` for optional boolean extension validation.
- Accepted `return_token_ids` when missing, null, `false`, or `true`.
- Rejected malformed non-boolean values such as `"true"`.

The extension is currently request-compatible only. Ferrite does not claim to
return token IDs. In the successful `llama-benchy` run, the tool reported:

```text
No token_ids in response, using local tokenization
```

## Red Test

The new route test failed before implementation:

```text
assertion `left == right` failed: {"error":{"code":null,"message":"unsupported chat completion field(s): return_token_ids","param":"return_token_ids","type":"invalid_request_error"}}
  left: 400
 right: 200
```

## Validation

```text
CARGO_TARGET_DIR=target/codex-llama-benchy-return-token-ids cargo test -p ferrite-server openai::chat_option_tests::chat_endpoint_accepts_return_token_ids_extension -- --nocapture
```

Result: 1 passed, 0 failed.

```text
CARGO_TARGET_DIR=target/codex-llama-benchy-return-token-ids cargo test -p ferrite-server openai::unsupported_tests::chat_endpoint_rejects_malformed_return_token_ids -- --nocapture
```

Result: 1 passed, 0 failed.

```text
CARGO_TARGET_DIR=target/codex-llama-benchy-return-token-ids cargo test -p ferrite-server openai::chat_option_tests -- --nocapture
CARGO_TARGET_DIR=target/codex-llama-benchy-return-token-ids cargo test -p ferrite-server openai::unsupported_tests -- --nocapture
CARGO_TARGET_DIR=target/codex-llama-benchy-return-token-ids cargo test -p ferrite-server openai::schema::neutral_options -- --nocapture
```

Results: 22 passed, 11 passed, and 9 passed.

```text
cargo fmt --all -- --check
git diff --check
```

Result: both exited cleanly after formatting.

Live external smoke:

```text
uvx llama-benchy ... --pp 32 --tg 16 --runs 1 --concurrency 1 --latency-mode none ...
```

Result: `llama-benchy` completed one streaming chat benchmark request and wrote
`documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-compat-smoke-after-return-token-ids.json`.

## Limits

This is not the full `llama-benchy` protocol, not the 256/512/1024-token
long-chat gate, and not proof that token IDs are returned. It only proves that
this external benchmark can complete a minimal streaming chat request against a
real local Ferrite model after the request-schema compatibility change.
