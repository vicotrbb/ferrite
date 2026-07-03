# Benchmark: x86_64 Paired Latency Cache Qwen 0.5B 1024

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Run the third bounded x86_64 paired measurement from the latency/cache
companion protocol:

- Ferrite's long-chat gate runs inside a bounded amd64 Kubernetes pod and
  provides correctness, generated-context identity, cache metadata, reconnect
  probes, and RSS samples.
- `llama-benchy` runs inside the same pod against the local Ferrite
  OpenAI-compatible server and provides an external client-side prefix-cache
  latency view without relying on a long Kubernetes port-forward.

This completes the x86_64 Qwen 0.5B paired 256/512/1024 smoke ladder. It does
not close the full Tier 1 long-chat gate, because the larger required Tier 1
models still need the same dedicated closure evidence.

## Environment

- Ferrite commit: `f03e2cd`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-paired-qwen05-1024`
- Node: `homelab-01`
- Pod IP: `10.42.248.252`
- Container image: `rust:1.96-bookworm`
- Architecture: `x86_64`
- CPU feature evidence: `/proc/cpuinfo` included `avx2`
- CPU request: `500m`
- CPU limit: `2`
- Memory request: `1Gi`
- Memory limit: `6Gi` (`memory.max=6442450944`)
- Ephemeral-storage request: `6Gi`
- Ephemeral-storage limit: `10Gi`
- Workspace size after source copy, model copy, release build, proof, uv cache,
  and in-pod `llama-benchy`: `542M`
- Pod cgroup memory current after proof and companion benchmark:
  `1576239104` bytes
- Pod cgroup memory peak after proof and companion benchmark:
  `1946292224` bytes
- Raw pod proof artifacts copied locally:
  `target/proof/x86-paired-qwen05-1024-2026-07-03/`

The staging API returned `ok` before pod creation. During the long proof,
control-plane calls saw transient interruptions including `etcdserver: leader
changed`, one `connect: connection refused`, and temporary API readiness
errors. The in-pod server and proof processes survived those control-plane
events. The pod was deleted after artifact collection, and a final
`kubectl --context staging get pod ferrite-avx2-paired-qwen05-1024 --ignore-not-found`
returned no pod output. Both staging nodes were Ready after cleanup.

## Model

- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Pod path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

## Binaries

The binaries were built inside the amd64 pod. `file` reported both as
`ELF 64-bit LSB pie executable, x86-64`.

- `target/release/ferrite-server` SHA256:
  `c6e52e0858d8676d54636c0ef004e3b17b6f9b2f03890a86fc5ca97d462b3bac`
- `target/release/ferrite-openai-long-chat-gate` SHA256:
  `74e495ab2cf63aa2d18899498ead0ce53c677d3b92618109eba28e79e9a1386c`

Build result:

```text
Finished `release` profile [optimized] target(s) in 41.57s
```

## Server

```sh
./target/release/ferrite-server \
  --bind 127.0.0.1:18194 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 1024 \
  --hard-max-tokens 2048 \
  --inference-wait-ms 30000 \
  --experimental-prefix-cache
```

Readiness:

```text
GET /health -> {"status":"ok","ready":true,"model":"Qwen2.5-0.5B-Instruct-Q4_K_M"}
```

Server PID for the long-chat RSS sampling: `1643`.

## Ferrite Gate

```sh
./target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --addr 127.0.0.1:18194 \
  --api-key local-secret \
  --models Qwen2.5-0.5B-Instruct-Q4_K_M \
  --token-lengths 1024 \
  --turns 4 \
  --probe-max-tokens 1024 \
  --rss-pid 1643 \
  --prompt-cache-key ferrite:paired:x86:qwen05:latency-cache-1024-current \
  --prompt-cache-trace \
  --require-cached-follow-ups \
  --expect-finish-reason length \
  --proof-log target/proof/x86-paired-qwen05-1024-2026-07-03/x86-paired-qwen05-1024-long-chat.log \
  --proof-exit-code target/proof/x86-paired-qwen05-1024-2026-07-03/x86-paired-qwen05-1024-long-chat.exit
```

The initial long `kubectl exec` stream dropped with websocket EOF. The in-pod
gate process survived, continued running, and exited `0`. The proof log contains
214 lines.

### Probe Results

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_error_probe_reconnect_generated_event=true
long_chat_error_probe_reconnect_started_new_generation=true
long_chat_error_probe_max_tokens=1024
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
long_chat_disconnect_probe_reconnect_generated_event=true
long_chat_disconnect_probe_reconnect_started_new_generation=true
long_chat_disconnect_probe_max_tokens=1024
```

### Scenario Results

| Turn | Prompt | Cached | Lookup | Context hash | Response hash | TTFT ms | Stream tok/s | RSS idle |
| ---: | ---: | ---: | --- | --- | --- | ---: | ---: | ---: |
| 1 | 43 | 0 | `miss` | `fnv64:13669ce34c14a412` | `fnv64:890bd91fd63ce8b0` | 13778 | 2.716768 | 459534336 |
| 2 | 1054 | 12 | `shared_prefix_hit` | `fnv64:890bd91fd63ce8b0` | `fnv64:d3b6392e4ebce4da` | 368863 | 1.269878 | 515764224 |
| 3 | 1054 | 16 | `shared_prefix_hit` | `fnv64:d3b6392e4ebce4da` | `fnv64:d3b6392e4ebce4da` | 369281 | 1.272351 | 542240768 |
| 4 | 1054 | 1054 | `exact_hit` | `fnv64:d3b6392e4ebce4da` | `fnv64:d3b6392e4ebce4da` | 295 | 2.420130 | 543682560 |

Every scenario reported:

```text
long_chat_result_finish_reason=length
long_chat_result_hit_token_limit=true
long_chat_result_streaming_content_chunks=1024
long_chat_result_streaming_token_id_chunks=1024
long_chat_result_streaming_token_ids=1024
long_chat_result_streaming_all_content_chunks_have_token_ids=true
```

Summary fields included:

```text
long_chat_summary_completed_scenarios=4
long_chat_summary_all_generated_follow_up_turns_cached=true
long_chat_summary_all_follow_up_turns_use_generated_context=true
long_chat_summary_all_generated_context_identities_match_previous_response=true
long_chat_summary_error_probe_completed=true
long_chat_summary_error_probe_reconnect_started_new_generation=true
long_chat_summary_disconnect_probe_completed=true
long_chat_summary_disconnect_probe_reconnect_started_new_generation=true
long_chat_summary_run_complete=true
```

## llama-benchy Companion

The accepted companion benchmark ran inside the proof pod against
`http://127.0.0.1:18194/v1` after installing `uv 0.11.26`.

```sh
/root/.local/bin/uvx llama-benchy \
  --base-url http://127.0.0.1:18194/v1 \
  --api-key local-secret \
  --model Qwen/Qwen2.5-0.5B-Instruct \
  --served-model-name Qwen2.5-0.5B-Instruct-Q4_K_M \
  --tokenizer Qwen/Qwen2.5-0.5B-Instruct \
  --pp 1024 \
  --tg 1024 \
  --depth 1024 \
  --runs 1 \
  --concurrency 1 \
  --latency-mode none \
  --no-warmup \
  --skip-coherence \
  --adapt-prompt \
  --enable-prefix-caching \
  --extra-body prompt_cache_key=ferrite:paired:x86:qwen05:benchy-1024-inpod-current \
  --format json \
  --save-result target/proof/x86-paired-qwen05-1024-2026-07-03/x86-paired-qwen05-1024-llama-benchy-inpod.json
```

The command exited `0`.

Raw JSON:
`documentation/benchmarks/2026-07-03-llama-benchy-x86-qwen-0-5b-paired-cache-1024.json`

Captured stdout:
`target/proof/x86-paired-qwen05-1024-2026-07-03/x86-paired-qwen05-1024-llama-benchy-inpod.stdout`

### llama-benchy Results

| Phase | Depth | Prompt | Generated | Concurrency | TG tok/s | TTFR ms | E2E TTFT ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| context | 1024 | 1024 | 1024 | 1 | 2.359277 | 42.047266 | 359177.165560 |
| inference | 1024 | 1024 | 1024 | 1 | 2.068329 | 41.905135 | 420147.330409 |

`llama-benchy` reported version `0.3.8`, timestamp
`2026-07-03 13:52:01Z`, `latency_mode=none`,
`prefix_caching_enabled=true`, and `max_concurrency=1`. It emitted the known
tokenizer warning for the full Sherlock Holmes source corpus length
(`143278 > 131072`), but the accepted run completed and produced non-null
benchmark rows.

## Server Lifecycle

The server emitted nine lifecycle lines:

```text
openai_stream_lifecycle request_id=stream-0 finish_reason=completed disconnect_point=none prompt_tokens_started=43 prompt_cancellation_polls=1075 generated_chunks=1024 generated_token_ids=1024 elapsed_ms=379092
openai_stream_lifecycle request_id=stream-1 finish_reason=cancelled disconnect_point=token_streaming prompt_tokens_started=0 prompt_cancellation_polls=0 generated_chunks=2 generated_token_ids=2 elapsed_ms=427
openai_stream_lifecycle request_id=stream-2 finish_reason=completed disconnect_point=none prompt_tokens_started=0 prompt_cancellation_polls=0 generated_chunks=1024 generated_token_ids=1024 elapsed_ms=362131
openai_stream_lifecycle request_id=stream-3 finish_reason=completed disconnect_point=none prompt_tokens_started=43 prompt_cancellation_polls=1075 generated_chunks=1024 generated_token_ids=1024 elapsed_ms=377286
openai_stream_lifecycle request_id=stream-4 finish_reason=completed disconnect_point=none prompt_tokens_started=1042 prompt_cancellation_polls=26050 generated_chunks=1024 generated_token_ids=1024 elapsed_ms=807164
openai_stream_lifecycle request_id=stream-5 finish_reason=completed disconnect_point=none prompt_tokens_started=1038 prompt_cancellation_polls=25950 generated_chunks=1024 generated_token_ids=1024 elapsed_ms=805595
openai_stream_lifecycle request_id=stream-6 finish_reason=completed disconnect_point=none prompt_tokens_started=0 prompt_cancellation_polls=0 generated_chunks=1024 generated_token_ids=1024 elapsed_ms=423530
openai_stream_lifecycle request_id=stream-7 finish_reason=completed disconnect_point=none prompt_tokens_started=1026 prompt_cancellation_polls=25650 generated_chunks=1024 generated_token_ids=1024 elapsed_ms=793265
openai_stream_lifecycle request_id=stream-8 finish_reason=completed disconnect_point=none prompt_tokens_started=1030 prompt_cancellation_polls=25750 generated_chunks=1024 generated_token_ids=1024 elapsed_ms=915254
```

`stream-1` is the intentional disconnect-probe cancellation. Streams 7 and 8
are the in-pod `llama-benchy` context and inference phases.

## Artifacts

| Artifact | Lines / bytes | SHA256 |
| --- | ---: | --- |
| `target/proof/x86-paired-qwen05-1024-2026-07-03/x86-paired-qwen05-1024-long-chat.log` | 214 lines | `0dd52a3fd5531fbdf0049871702e758c9a693dc95672dc93cfdbb760f349c116` |
| `target/proof/x86-paired-qwen05-1024-2026-07-03/x86-paired-qwen05-1024-long-chat.exit` | 2 bytes | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |
| `target/proof/x86-paired-qwen05-1024-2026-07-03/x86-paired-qwen05-1024-llama-benchy-inpod.json` | 3252 bytes | `3561e23b1b3185e2b9c2b64f746eea98b1b85d5cd7a2faefc7f3dfd60d139b77` |
| `target/proof/x86-paired-qwen05-1024-2026-07-03/x86-paired-qwen05-1024-llama-benchy-inpod.stdout` | 42 lines | `54c65955d1d9168792d01ea8b925bde624aabc47dc84e2a96cfa372e097c3dc4` |
| `target/proof/x86-paired-qwen05-1024-2026-07-03/server.log` | 9 lines | `2827f2f2efde5d5c323819a9b1875d2f9b7e8d7e8c3eebb47f610ddee3c29179` |
| `target/proof/x86-paired-qwen05-1024-2026-07-03/cgroup-memory-after.txt` | 3 lines | `931bd1ed33191962e644842a73169843374f047bcb1fd47310be8ded0a34b276` |

## Interpretation

The x86_64 1024-token paired run reproduces the local 1024 fixed-point cache
mechanism on the current lifecycle server:

- Ferrite's gate proved generated assistant context was carried across turns
  and that all generated-context identity links matched previous responses.
- Turns 2 and 3 were shallow shared-prefix hits: `12 / 1054` and `16 / 1054`
  cached prompt tokens, with TTFT around 369 seconds.
- Turn 3 generated the same response identity as its assistant context.
- Turn 4 reused the full prompt: `1054 / 1054` cached prompt tokens,
  `lookup=exact_hit`, and TTFT collapsed to `295` ms.
- The in-pod `llama-benchy` companion completed the different
  system-context-prefix shape at depth 1024, prompt 1024, and generation 1024
  without relying on Kubernetes port-forward lifetime.
- Server RSS stayed bounded under the 6 GiB pod limit. Cgroup peak across
  build, proof, `uv`, tokenizer tooling, and companion benchmark was
  `1946292224` bytes.

This completes the x86_64 paired 256/512/1024 Qwen 0.5B ladder. The result
supports the companion protocol split: Ferrite's gate explains generated-chat
correctness and cache identity, while `llama-benchy` supplies external
OpenAI-compatible latency trend rows.

## Limits

This run does not prove:

- Qwen2.5 1.5B Q8_0 or Q6_K 1024-token paired behavior;
- SmolLM2 1.7B 256/512/1024 generated-output behavior;
- stop/EOS behavior for the 1024 length lane beyond the separate explicit-stop
  and natural-EOS proof notes;
- high-concurrency behavior;
- long-running RSS stability beyond this bounded proof;
- that `llama-benchy` can replace Ferrite's long-chat gate.
