# Benchmark: x86_64 SmolLM2 135M Long-Chat Full Matrix

Date: 2026-07-03 UTC, 2026-07-02 local time

## Purpose

Run the dedicated OpenAI-compatible long-chat gate on x86_64 at 256, 512, and
1024 streaming response tokens using the new durable proof artifact flags.

This run exercises:

- `GET /health`;
- `GET /v1/models`;
- `POST /v1/chat/completions` streaming;
- 256, 512, and 1024-token streaming responses;
- four-turn generated-context conversations;
- RSS samples before, after, and after idle;
- per-token latency and time-to-first-token summaries;
- unauthorized request recovery;
- client disconnect/reconnect behavior;
- OpenAI-compatible streaming token-id summaries;
- shared-prefix cache evidence through `--require-cached-follow-ups`;
- durable proof files through `--proof-log` and `--proof-exit-code`.

## Environment

- Ferrite commit: `26b9edc827a1d7be97b6b7848fb5895bf8e2a1df`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-smollm135-long-chat-full`
- Node: `homelab-01`
- Container image: `rust:1.96-bookworm`
- Architecture: `x86_64`
- CPU: AMD Ryzen 7 5825U with Radeon Graphics
- CPU feature evidence: `avx` and `avx2` present in `lscpu`
- CPU request: `500m`
- CPU limit: `2`
- Memory request: `512Mi`
- Memory limit: `3Gi`
- Ephemeral-storage request: `5Gi`
- Ephemeral-storage limit: `8Gi`
- Pod cgroup memory peak after build and proof: `778801152` bytes
- Workspace size after source copy, model copy, release build, and proof: `262M`
- Raw proof log copied locally:
  `target/proof/x86-smollm135-full-matrix-2026-07-03.log`
- Raw server log copied locally:
  `target/proof/x86-smollm135-full-matrix-server-2026-07-03.log`

## Model

- Model: `SmolLM2-135M-Instruct-Q4_K_M`
- Served model id: `smollm2-135m-q4_k_m`
- Pod path: `target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf`
- SHA256:
  `2e8040ceae7815abe0dcb3540b9995eaa1fa0d2ca9e797d0a635ae4433c68c2d`

## Binaries

- `target/release/ferrite-server` SHA256:
  `d485b838c555dc052bebcf562ecce73fabee7ee987cc20ad35df2e0df7e3d3e5`
- `target/release/ferrite-openai-long-chat-gate` SHA256:
  `6fc9f196b494b5e298381dc4a330aab3c9e0cd5df86bc1e2a4511eb2ea9bbe53`

## Build

```sh
kubectl --context staging exec ferrite-avx2-smollm135-long-chat-full -- sh -lc \
  'export PATH=/usr/local/cargo/bin:$PATH; cd /work/ferrite && cargo build -p ferrite-server --release --bins'
```

Result:

```text
Finished `release` profile [optimized] target(s) in 48.24s
```

## Server

```sh
./target/release/ferrite-server \
  --bind 127.0.0.1:18080 \
  --model target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf \
  --model-id smollm2-135m-q4_k_m \
  --api-key local-secret \
  --default-max-tokens 64 \
  --hard-max-tokens 1024 \
  --inference-wait-ms 30000 \
  --experimental-prefix-cache
```

Readiness checks:

```text
GET /health -> {"status":"ok","ready":true,"model":"smollm2-135m-q4_k_m"}
GET /v1/models -> {"object":"list","data":[{"id":"smollm2-135m-q4_k_m","object":"model","created":0,"owned_by":"ferrite"}]}
```

## Gate

```sh
./target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --addr 127.0.0.1:18080 \
  --api-key local-secret \
  --models smollm2-135m-q4_k_m \
  --token-lengths 256,512,1024 \
  --turns 4 \
  --probe-max-tokens 1024 \
  --rss-pid 1641 \
  --prompt-cache-key long-chat:smollm135:x86-full \
  --require-cached-follow-ups \
  --expect-finish-reason length \
  --proof-log target/proof/smollm135-full-matrix.log \
  --proof-exit-code target/proof/smollm135-full-matrix.exit
```

The durable exit-code file contained:

```text
0
```

## Probe Results

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_error_probe_max_tokens=1024
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
long_chat_disconnect_probe_reconnect_generated_event=true
long_chat_disconnect_probe_reconnect_started_new_generation=true
long_chat_disconnect_probe_max_tokens=1024
```

## Scenario Results

| Turn | Tokens | Context | Prompt | Cached | TTFT ms | Decode tok/s | Stream tok/s | RSS before | RSS after |
| ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 256 | seed | 48 | 0 | 4788 | 9.386903 | 8.016190 | 173678592 | 173678592 |
| 1 | 512 | seed | 48 | 48 | 26 | 8.830834 | 8.844055 | 173678592 | 173678592 |
| 1 | 1024 | seed | 48 | 48 | 23 | 7.779132 | 7.785339 | 173678592 | 174989312 |
| 2 | 256 | generated | 282 | 14 | 30970 | 8.221076 | 4.137807 | 174989312 | 175120384 |
| 2 | 512 | generated | 531 | 263 | 31843 | 7.130011 | 4.949237 | 175120384 | 217980928 |
| 2 | 1024 | generated | 1028 | 512 | 72592 | 5.142857 | 3.772486 | 217980928 | 288366592 |
| 3 | 256 | generated | 289 | 15 | 30575 | 7.885861 | 4.076887 | 288366592 | 288366592 |
| 3 | 512 | generated | 545 | 526 | 2514 | 7.036901 | 6.815101 | 288366592 | 288366592 |
| 3 | 1024 | generated | 1057 | 15 | 132944 | 5.191453 | 3.104258 | 288366592 | 288366592 |
| 4 | 256 | generated | 276 | 33 | 25649 | 8.294852 | 4.547732 | 288366592 | 288366592 |
| 4 | 512 | generated | 545 | 34 | 58460 | 6.936967 | 3.878489 | 288366592 | 288366592 |
| 4 | 1024 | generated | 1004 | 33 | 127966 | 5.222556 | 3.163201 | 288366592 | 288366592 |

Every row reported:

```text
long_chat_result_finish_reason=length
long_chat_result_hit_token_limit=true
long_chat_result_streaming_all_content_chunks_have_token_ids=true
```

## Integrated Summary

```text
long_chat_summary_planned_scenarios=12
long_chat_summary_completed_scenarios=12
long_chat_summary_all_finish_reasons_present=true
long_chat_summary_all_usage_accounting_valid=true
long_chat_summary_all_token_limit_status_present=true
long_chat_summary_any_token_limit_hit=true
long_chat_summary_prompt_cache_key_present=true
long_chat_summary_cached_follow_ups_required=true
long_chat_summary_any_cached_prompt_tokens=true
long_chat_summary_generated_follow_up_turns=9
long_chat_summary_cached_generated_follow_up_turns=9
long_chat_summary_uncached_generated_follow_up_turns=0
long_chat_summary_all_generated_follow_up_turns_cached=true
long_chat_summary_all_follow_up_turns_use_generated_context=true
long_chat_summary_all_timing_present=true
long_chat_summary_streaming_token_ids_required=true
long_chat_summary_all_streaming_token_id_summaries_present=true
long_chat_summary_all_streaming_content_chunks_have_token_ids=true
long_chat_summary_rss_required=true
long_chat_summary_all_rss_present=true
long_chat_summary_error_probe_required=true
long_chat_summary_error_probe_completed=true
long_chat_summary_disconnect_probe_required=true
long_chat_summary_disconnect_probe_completed=true
long_chat_summary_disconnect_probe_reconnect_started_new_generation=true
long_chat_summary_run_complete=true
```

## Operational Notes

During polling, the Kubernetes API intermittently returned:

```text
etcdserver: request timed out
apiserver not ready
etcdserver: leader changed
connect: connection refused
```

The pod-side proof continued through those control-plane interruptions. The new
`--proof-log` and `--proof-exit-code` options preserved the run evidence and
made short polling execs sufficient.

## Interpretation

This closes an x86_64 full-matrix proof for the smallest local model tier:
SmolLM2-135M Q4_K_M completed the 256/512/1024 generated-context long-chat gate
through Ferrite's OpenAI-compatible HTTP server with RSS, per-token latency,
token IDs, error probe, disconnect/reconnect probe, and cached follow-up checks.

The result is intentionally scoped. It does not prove Tier 1 0.5B-1.7B full
matrix behavior, natural memory quality, explicit stop/EOS behavior in the same
matrix, high-concurrency serving, or long-running leak freedom.

## Next Step

Use the same durable proof artifact pattern for the next Tier 1 x86 full-matrix
candidate. The most practical next target is `Qwen2.5-0.5B-Instruct-Q4_K_M`
with the same 256/512/1024 gate and `--require-cached-follow-ups`.
