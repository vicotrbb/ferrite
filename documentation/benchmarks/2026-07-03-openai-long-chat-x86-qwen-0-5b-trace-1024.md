# Benchmark: x86_64 Qwen 0.5B Prompt-Cache Trace 1024

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Run the OpenAI-compatible long-chat gate on x86_64 with
`Qwen2.5-0.5B-Instruct-Q4_K_M`, 1024-token streaming responses, generated
follow-up context, and opt-in prompt-cache trace output.

This run specifically tests whether the new trace explains the 1024-token
cache-depth and TTFT instability seen in the previous x86 full matrix.

## Environment

- Ferrite commit: `7816421`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-qwen05-trace-1024`
- Node: `homelab-01`
- Pod IP: `10.42.248.241`
- Container image: `rust:1.96-bookworm`
- Architecture: `x86_64`
- CPU feature evidence: `/proc/cpuinfo` included `avx` and `avx2`
- CPU request: `500m`
- CPU limit: `2`
- Memory request: `1Gi`
- Memory limit: `6Gi` (`memory.max=6442450944`)
- Ephemeral-storage request: `6Gi`
- Ephemeral-storage limit: `10Gi`
- Workspace size after source copy, model copy, release build, and proof:
  `541M`
- Pod cgroup memory current after proof: `1131909120` bytes
- Pod cgroup memory peak after build and proof: `1499168768` bytes
- Raw proof artifacts copied locally:
  `target/proof/x86-qwen05-trace-1024-2026-07-03/`

The staging API was checked before pod creation:

```text
kubectl config current-context -> staging
kubectl --context staging get --raw=/readyz -> ok
```

The pod was deleted after artifact collection. A final
`kubectl --context staging get pod ferrite-avx2-qwen05-trace-1024 --ignore-not-found`
returned no pod output.

## Model

- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Pod path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

## Binaries

The binaries were built inside the amd64 pod. `file` reported both as
`ELF 64-bit LSB pie executable, x86-64`.

- `target/release/ferrite-server` SHA256:
  `14f4c0858d5cc2a812f6c073f17cef9249fa89bf92ed28092cf0ec49c1055f34`
- `target/release/ferrite-openai-long-chat-gate` SHA256:
  `a72f113816f762e1ba1a81016d45aff1bdd2c67fcf95a4453ab6b8fce436b11d`

Build command:

```sh
kubectl --context staging exec ferrite-avx2-qwen05-trace-1024 -- sh -lc \
  'export PATH=/usr/local/cargo/bin:$PATH; cd /work/ferrite && cargo build -p ferrite-server --release --bins'
```

Result:

```text
Finished `release` profile [optimized] target(s) in 41.44s
```

## Server

```sh
./target/release/ferrite-server \
  --bind 127.0.0.1:18190 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 1024 \
  --hard-max-tokens 2048 \
  --inference-wait-ms 30000 \
  --experimental-prefix-cache
```

Readiness checks:

```text
GET /health -> {"status":"ok","ready":true,"model":"Qwen2.5-0.5B-Instruct-Q4_K_M"}
GET /v1/models -> {"object":"list","data":[{"id":"Qwen2.5-0.5B-Instruct-Q4_K_M","object":"model","created":0,"owned_by":"ferrite"}]}
```

Clean-run server PID for RSS sampling: `1777`.

## Gate

```sh
./target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --addr 127.0.0.1:18190 \
  --api-key local-secret \
  --models Qwen2.5-0.5B-Instruct-Q4_K_M \
  --token-lengths 1024 \
  --turns 4 \
  --probe-max-tokens 1024 \
  --rss-pid 1777 \
  --prompt-cache-key ferrite:long-chat:qwen05:x86-trace-1024-clean \
  --prompt-cache-trace \
  --require-cached-follow-ups \
  --expect-finish-reason length \
  --proof-log target/proof/x86-qwen05-trace-1024.log \
  --proof-exit-code target/proof/x86-qwen05-trace-1024.exit
```

The gate was launched detached in the pod and wrote:

```text
target/proof/x86-qwen05-trace-1024.exit -> 0
target/proof/x86-qwen05-trace-1024.process-exit -> 0
```

Copied artifact line counts:

```text
185 target/proof/x86-qwen05-trace-1024-2026-07-03/x86-qwen05-trace-1024.log
185 target/proof/x86-qwen05-trace-1024-2026-07-03/x86-qwen05-trace-1024.stdout
0   target/proof/x86-qwen05-trace-1024-2026-07-03/x86-qwen05-trace-1024-server.log
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

| Turn | Context | Prompt | Cached | Lookup | Prompt hash | Selected entry hash | Shared prefix | TTFT ms | Decode tok/s | Stream tok/s | RSS before | RSS after | RSS idle |
| ---: | --- | ---: | ---: | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | seed | 43 | 0 | miss | `fnv64:92585af239e73208` | | 0 | 14381 | 2.756055 | 2.655939 | 456269824 | 458104832 | 458104832 |
| 2 | generated | 1054 | 12 | shared_prefix_hit | `fnv64:93e2cf81835f98a6` | `fnv64:92585af239e73208` | 12 | 378227 | 2.288260 | 1.241327 | 458104832 | 515514368 | 515514368 |
| 3 | generated | 1054 | 16 | shared_prefix_hit | `fnv64:2249cfc489e572a7` | `fnv64:93e2cf81835f98a6` | 16 | 374869 | 2.216607 | 1.224850 | 515514368 | 542081024 | 542081024 |
| 4 | generated | 1054 | 1054 | exact_hit | `fnv64:2249cfc489e572a7` | `fnv64:2249cfc489e572a7` | 1054 | 308 | 2.388761 | 2.389374 | 542081024 | 515596288 | 515596288 |

Every row reported:

```text
long_chat_result_finish_reason=length
long_chat_result_hit_token_limit=true
long_chat_result_streaming_all_content_chunks_have_token_ids=true
```

## Integrated Summary

```text
long_chat_summary_planned_scenarios=4
long_chat_summary_completed_scenarios=4
long_chat_summary_all_finish_reasons_present=true
long_chat_summary_all_usage_accounting_valid=true
long_chat_summary_all_token_limit_status_present=true
long_chat_summary_any_token_limit_hit=true
long_chat_summary_prompt_cache_key_present=true
long_chat_summary_cached_follow_ups_required=true
long_chat_summary_any_cached_prompt_tokens=true
long_chat_summary_generated_follow_up_turns=3
long_chat_summary_cached_generated_follow_up_turns=3
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

## Interpretation

The x86_64 1024-token trace proves that the previous high-TTFT generated rows
are explainable from cache depth:

- turn 2 reused only 12 of 1054 prompt tokens and TTFT was 378227 ms;
- turn 3 reused only 16 of 1054 prompt tokens and TTFT was 374869 ms;
- turn 4 reused all 1054 prompt tokens and TTFT collapsed to 308 ms.

The selected-entry hashes make the reuse chain explicit. Turn 2 selected the
seed prompt hash, turn 3 selected turn 2, and turn 4 exactly matched turn 3.
This confirms the trace output is sufficient to diagnose the cache path without
manual generated-text inspection.

This is not an optimization proof. It shows that low cache depth remains the
dominant latency problem for generated-context 1024-token chats on this bounded
x86 pod. Decode throughput stayed in a narrower range, from 2.216607 to
2.756055 tok/s, while TTFT varied from 308 ms to 378227 ms.

RSS stayed bounded for this single run. Server idle RSS peaked at 542081024
bytes after turn 3 and returned to 515596288 bytes after the exact-hit turn.
The pod cgroup peak was 1499168768 bytes. This is single-run bounded RSS
evidence, not a leak-freedom claim.

## Operational Notes

Initial detached launch attempts used incorrect shell grouping and produced
duplicate gate processes. Those processes were stopped, the server was
restarted, partial proof files were removed, and the clean run used a fresh
server PID (`1777`) and prompt-cache key
`ferrite:long-chat:qwen05:x86-trace-1024-clean`.

During cleanup, the Kubernetes API briefly returned connection-refused and EOF
errors. A retry returned `ok` from `/readyz`, the pod deletion completed, and
the final `get pod --ignore-not-found` check returned no pod.

## Limits

This run does not prove:

- 256/512-token traced x86 behavior;
- multi-run steady-state cache stability;
- a cache optimization;
- stop/EOS traced behavior;
- larger Tier 1 model trace behavior;
- production leak freedom.

## Next Step

Implement or test the next theory against the trace evidence: why turn 4 exactly
matched turn 3 while turns 2 and 3 shared only 12 to 16 tokens with their
selected entries. The most likely next diagnostic is prompt/rendered-token
comparison for generated assistant context, followed by a cache-key or prompt
serialization change only if the token evidence supports it.
