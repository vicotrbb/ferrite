# Benchmark: x86_64 Qwen 0.5B Long-Chat Full Matrix

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Run the dedicated OpenAI-compatible long-chat gate on x86_64 with
Qwen2.5-0.5B-Instruct Q4_K_M at 256, 512, and 1024 streaming response tokens.

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

- Ferrite commit: `20bfaebc3c60018bbb0dd24099229e6cd90b9ea3`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-qwen05-long-chat-full`
- Node: `homelab-01`
- Container image: `rust:1.96-bookworm`
- Architecture: `x86_64`
- CPU: AMD Ryzen 7 5825U with Radeon Graphics
- CPU feature evidence: `avx2` present in `/proc/cpuinfo`; `AVX2_COUNT=16`
- CPU request: `500m`
- CPU limit: `2`
- Memory request: `768Mi`
- Memory limit: `4Gi`
- Ephemeral-storage request: `6Gi`
- Ephemeral-storage limit: `10Gi`
- Pod cgroup memory peak after build and proof: `1483046912` bytes
- Workspace size after source copy, model copy, release build, and proof: `541M`
- Raw proof log copied locally:
  `target/proof/x86-qwen05-full-matrix-2026-07-03.log`
- Raw proof stdout copied locally:
  `target/proof/x86-qwen05-full-matrix-2026-07-03.stdout`
- Raw proof stderr copied locally:
  `target/proof/x86-qwen05-full-matrix-2026-07-03.stderr`
- Raw server log copied locally:
  `target/proof/x86-qwen05-full-matrix-server-2026-07-03.log`

## Model

- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Served model id: `qwen2.5-0.5b-q4_k_m`
- Pod path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

## Binaries

- `target/release/ferrite-server` SHA256:
  `d485b838c555dc052bebcf562ecce73fabee7ee987cc20ad35df2e0df7e3d3e5`
- `target/release/ferrite-openai-long-chat-gate` SHA256:
  `6fc9f196b494b5e298381dc4a330aab3c9e0cd5df86bc1e2a4511eb2ea9bbe53`

## Build

```sh
kubectl --context staging exec ferrite-avx2-qwen05-long-chat-full -- sh -lc \
  'export PATH=/usr/local/cargo/bin:$PATH; cd /work/ferrite && cargo build -p ferrite-server --release --bins'
```

Result:

```text
Finished `release` profile [optimized] target(s) in 38.65s
```

## Server

```sh
./target/release/ferrite-server \
  --bind 127.0.0.1:18080 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id qwen2.5-0.5b-q4_k_m \
  --api-key local-secret \
  --default-max-tokens 64 \
  --hard-max-tokens 1024 \
  --inference-wait-ms 30000 \
  --experimental-prefix-cache
```

Readiness checks:

```text
GET /health -> {"status":"ok","ready":true,"model":"qwen2.5-0.5b-q4_k_m"}
GET /v1/models -> {"object":"list","data":[{"id":"qwen2.5-0.5b-q4_k_m","object":"model","created":0,"owned_by":"ferrite"}]}
```

## Gate

```sh
./target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --addr 127.0.0.1:18080 \
  --api-key local-secret \
  --models qwen2.5-0.5b-q4_k_m \
  --token-lengths 256,512,1024 \
  --turns 4 \
  --probe-max-tokens 1024 \
  --rss-pid 1641 \
  --prompt-cache-key long-chat:qwen05:x86-full \
  --require-cached-follow-ups \
  --expect-finish-reason length \
  --proof-log target/proof/qwen05-full-matrix.log \
  --proof-exit-code target/proof/qwen05-full-matrix.exit
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

| Turn | Tokens | Context | Prompt | Cached | TTFT ms | Decode tok/s | Stream tok/s | RSS before | RSS after | Elapsed ms |
| ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 256 | seed | 43 | 0 | 13805 | 3.037768 | 2.620353 | 455041024 | 455041024 | 100093 |
| 1 | 512 | seed | 43 | 43 | 85 | 2.958022 | 2.962332 | 455041024 | 443342848 | 175191 |
| 1 | 1024 | seed | 43 | 43 | 80 | 2.760184 | 2.762283 | 443342848 | 457760768 | 373092 |
| 2 | 256 | generated | 286 | 12 | 93956 | 2.702199 | 1.361991 | 457760768 | 457760768 | 190709 |
| 2 | 512 | generated | 542 | 269 | 94417 | 2.646792 | 1.782119 | 457760768 | 479911936 | 289877 |
| 2 | 1024 | generated | 1054 | 525 | 202997 | 2.180737 | 1.524019 | 479911936 | 538120192 | 674587 |
| 3 | 256 | generated | 286 | 14 | 90134 | 2.875938 | 1.434557 | 538120192 | 523468800 | 181163 |
| 3 | 512 | generated | 542 | 306 | 82497 | 2.674020 | 1.872470 | 523468800 | 529760256 | 275986 |
| 3 | 1024 | generated | 1054 | 16 | 372787 | 2.283376 | 1.248104 | 529760256 | 558071808 | 823267 |
| 4 | 256 | generated | 286 | 14 | 91124 | 2.863183 | 1.423545 | 558071808 | 558071808 | 182588 |
| 4 | 512 | generated | 542 | 20 | 176883 | 2.656111 | 1.387813 | 558071808 | 558071808 | 371663 |
| 4 | 1024 | generated | 1054 | 1054 | 277 | 2.375039 | 2.375828 | 558071808 | 558071808 | 433453 |

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

During polling, the Kubernetes API intermittently returned transient transport
errors, including refused connections and unexpected EOF. The pod-side proof
continued and wrote a durable log plus durable exit-code file.

After artifact collection, the staging pod was deleted and a follow-up
`kubectl --context staging get pod ferrite-avx2-qwen05-long-chat-full --ignore-not-found`
returned no pod.

The copied proof log and stdout each contain 410 lines. The copied proof stderr
and server log are empty.

## Interpretation

This closes an x86_64 full-matrix proof for Qwen2.5-0.5B Q4_K_M through
Ferrite's OpenAI-compatible HTTP server. The run covered real streaming,
256/512/1024-token responses, four turns per token-length lane, generated
follow-up context, reconnect/error probes, token IDs, RSS samples, finish
reasons, and usage accounting.

The result is not a performance endorsement. It proves the path works, but it
also exposes long user-visible latency in generated follow-up turns. The worst
observed TTFT was 372787 ms on turn 3 / 1024 when only 16 prompt tokens were
reported cached. The best generated 1024-token follow-up TTFT was 277 ms on
turn 4 / 1024 when all 1054 prompt tokens were reported cached.

That contrast makes prefix-cache stability the next high-value theory target.
Decode throughput stayed comparatively narrow, roughly 2.18 to 3.04 tok/s,
while TTFT varied by three orders of magnitude.

## Next Step

Document and test a focused theory for generated-context prefix-cache
instability. The diagnostic should isolate why some lanes reuse nearly the full
prompt while adjacent generated follow-ups reuse only a few prompt tokens.
