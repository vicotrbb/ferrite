# Benchmark: OpenAI Throughput Token ID Observability Smoke

Date: 2026-07-02

## Purpose

Verify that Ferrite's own OpenAI throughput client reports streaming token-id
coverage against a real local model.

This is a proof-client observability smoke. It is not the full long-chat gate.

## Environment

- Ferrite commit: `5170bf4`
- Host: local macOS development machine
- Server: local Ferrite server on `127.0.0.1:18080`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Model path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`

## Server Command

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

## Client Command

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

## Result

```text
openai_http_streaming_chat_completion_requests=1
elapsed_ms=48016
streaming_token_events=9
streaming_time_to_first_token_ms=29809
streaming_total_elapsed_ms=48014
streaming_tokens_per_second=0.187443
streaming_finish_reason=length
streaming_text_bytes=40
streaming_content_chunks=8
streaming_token_id_chunks=8
streaming_token_ids=8
streaming_all_content_chunks_have_token_ids=true
streaming_usage_prompt_tokens=13
streaming_usage_cached_prompt_tokens=0
streaming_usage_completion_tokens=8
streaming_usage_total_tokens=21
```

## Interpretation

Ferrite's project-native throughput client can now observe and report that every
no-stop streaming content chunk carried token IDs for this real-model smoke.

This strengthens the long-chat proof path because future no-stop gate runs can
fail completion when token-id summaries are absent or incomplete.

## Limits

This run used only 8 generated tokens and one request. It does not prove the
full 256/512/1024 long-chat matrix, RSS stability, reconnect behavior,
stop/EOS behavior, or production throughput.
