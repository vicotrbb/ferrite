# OpenAI Stream Token ID Observability

## Goal

Make Ferrite's own OpenAI throughput and long-chat proof clients report whether
streaming content chunks include generated token IDs.

## Context

The chat streaming endpoint now emits `token_ids` on no-stop content chunks, and
`llama-benchy` can consume that shape. Ferrite's internal proof clients still
only reported text, usage, finish reason, token timing, and RSS. That left the
token-id compatibility behavior visible only through external tool output.

## Change

- Added a focused `streaming_token_ids` throughput-client parser module.
- Recorded content chunk count, token-id chunk count, total token IDs, and
  whether every content chunk has token IDs.
- Added the token-id summary to `ThroughputResult` and formatted throughput
  output.
- Added long-chat scenario result fields for token-id evidence.
- Added long-chat run-summary fields for token-id completeness.
- Made no-stop long-chat run completion require token-id summaries and complete
  content-chunk coverage. Stop-filtered runs do not require token IDs because
  filtered visible text may not map exactly to original model token IDs.

## Red Tests

The first parser test failed before implementation with:

```text
cannot find type `StreamingTokenIdsSummary` in this scope
```

The long-chat scenario-result test then failed because the formatter omitted:

```text
long_chat_result_streaming_content_chunks=...
long_chat_result_streaming_token_id_chunks=...
long_chat_result_streaming_token_ids=...
long_chat_result_streaming_all_content_chunks_have_token_ids=...
```

The integrated run-summary test then failed because the summary omitted:

```text
long_chat_summary_streaming_token_ids_required=...
long_chat_summary_all_streaming_token_id_summaries_present=...
long_chat_summary_all_streaming_content_chunks_have_token_ids=...
```

## Validation

```text
CARGO_TARGET_DIR=target/codex-token-id-summary cargo test -p ferrite-server throughput_client::tests -- --nocapture
```

Result: 47 passed, 0 failed.

```text
CARGO_TARGET_DIR=target/codex-token-id-summary cargo test -p ferrite-server --test long_chat_gate -- --nocapture
```

Result: 35 passed, 0 failed.

```text
cargo fmt --all -- --check
git diff --check
```

Result: both exited cleanly.

## Live Smoke

Server:

```sh
CARGO_TARGET_DIR=target/codex-token-id-summary \
  cargo run -p ferrite-server --bin ferrite-server -- \
  --bind 127.0.0.1:18080 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --default-max-tokens 8 \
  --hard-max-tokens 32
```

Readiness:

```text
200 {"status":"ok","ready":true,"model":"Qwen2.5-0.5B-Instruct-Q4_K_M"}
```

Client:

```sh
CARGO_TARGET_DIR=target/codex-token-id-summary \
  cargo run -p ferrite-server --bin ferrite-openai-throughput -- \
  --addr 127.0.0.1:18080 \
  --endpoint chat-completions \
  --model Qwen2.5-0.5B-Instruct-Q4_K_M \
  --prompt "Write one short sentence about CPU inference." \
  --max-tokens 8 \
  --requests 1 \
  --concurrency 1 \
  --stream \
  --stream-usage
```

Token-id evidence:

```text
streaming_content_chunks=8
streaming_token_id_chunks=8
streaming_token_ids=8
streaming_all_content_chunks_have_token_ids=true
```

## Limits

This adds observability and gate enforcement for no-stop streams. It does not
run the full 256/512/1024 long-chat matrix, does not add token IDs to
stop-filtered streams, and does not prove production throughput.
