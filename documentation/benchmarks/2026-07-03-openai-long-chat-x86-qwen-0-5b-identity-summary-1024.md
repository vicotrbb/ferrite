# Benchmark: x86_64 Qwen 0.5B Identity Summary Gate 1024

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Run the OpenAI-compatible long-chat gate on x86_64 with
`Qwen2.5-0.5B-Instruct-Q4_K_M`, 1024-token streaming responses, generated
follow-up context, prompt-cache tracing, generated-context identity summaries,
RSS sampling, unauthorized/reconnect probing, and disconnect/reconnect probing.

This reruns the earlier x86 1024-token trace after the gate learned to prove
that each generated assistant context matches the previous streamed response.

## Environment

- Ferrite commit: `4096d55`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-qwen05-identity-1024`
- Node: `homelab-01`
- Pod IP: `10.42.248.205`
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
- Pod cgroup memory current after proof: `1002254336` bytes
- Pod cgroup memory peak after build and proof: `1462358016` bytes
- Raw proof artifacts copied locally:
  `target/proof/x86-qwen05-identity-summary-1024-2026-07-03/`

The staging API was checked before pod creation:

```text
kubectl config current-context -> staging
kubectl --context staging get --raw=/readyz -> ok
```

The pod was deleted after artifact collection. A final
`kubectl --context staging get pod ferrite-avx2-qwen05-identity-1024 --ignore-not-found`
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
  `fa71c1ced92ef06697c1220e313dfdab6faf08da6d8851301e3013793ccb2727`
- `target/release/ferrite-openai-long-chat-gate` SHA256:
  `d1e9fbed42b9e65f950738d38e20a57162dec2a2c8078137dcaecfd598d1ba62`

Build command:

```sh
kubectl --context staging exec ferrite-avx2-qwen05-identity-1024 -- sh -lc \
  'export PATH=/usr/local/cargo/bin:$PATH; cd /work/ferrite && cargo build -p ferrite-server --release --bins'
```

Result:

```text
Finished `release` profile [optimized] target(s) in 41.01s
```

## Server

```sh
./target/release/ferrite-server \
  --bind 127.0.0.1:18191 \
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

Server PID for RSS sampling: `1657`.

## Gate

```sh
./target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --addr 127.0.0.1:18191 \
  --api-key local-secret \
  --models Qwen2.5-0.5B-Instruct-Q4_K_M \
  --token-lengths 1024 \
  --turns 4 \
  --probe-max-tokens 1024 \
  --rss-pid 1657 \
  --prompt-cache-key ferrite:long-chat:qwen05:x86-identity-summary-1024 \
  --prompt-cache-trace \
  --require-cached-follow-ups \
  --expect-finish-reason length \
  --proof-log target/proof/x86-qwen05-identity-summary-1024.log \
  --proof-exit-code target/proof/x86-qwen05-identity-summary-1024.exit
```

The gate was launched detached in the pod and wrote:

```text
target/proof/x86-qwen05-identity-summary-1024.exit -> 0
target/proof/x86-qwen05-identity-summary-1024.process-exit -> 0
```

Copied artifact line counts:

```text
210 target/proof/x86-qwen05-identity-summary-1024-2026-07-03/x86-qwen05-identity-summary-1024.log
210 target/proof/x86-qwen05-identity-summary-1024-2026-07-03/x86-qwen05-identity-summary-1024.stdout
0   target/proof/x86-qwen05-identity-summary-1024-2026-07-03/x86-qwen05-identity-summary-1024-server.log
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

| Turn | Context source | Context hash | Response hash | Prompt | Cached | Lookup | Prompt hash | Selected entry hash | TTFT ms | Decode tok/s | Stream tok/s | RSS before | RSS after | RSS idle |
| ---: | --- | --- | --- | ---: | ---: | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | seed | `fnv64:13669ce34c14a412` | `fnv64:890bd91fd63ce8b0` | 43 | 0 | `miss` | `fnv64:92585af239e73208` | | 13989 | 2.743969 | 2.647404 | 455208960 | 457961472 | 457961472 |
| 2 | generated | `fnv64:890bd91fd63ce8b0` | `fnv64:d3b6392e4ebce4da` | 1054 | 12 | `shared_prefix_hit` | `fnv64:93e2cf81835f98a6` | `fnv64:92585af239e73208` | 382172 | 2.300758 | 1.239055 | 457961472 | 513273856 | 513273856 |
| 3 | generated | `fnv64:d3b6392e4ebce4da` | `fnv64:d3b6392e4ebce4da` | 1054 | 16 | `shared_prefix_hit` | `fnv64:2249cfc489e572a7` | `fnv64:93e2cf81835f98a6` | 371694 | 2.286223 | 1.250617 | 513273856 | 542240768 | 542240768 |
| 4 | generated | `fnv64:d3b6392e4ebce4da` | `fnv64:d3b6392e4ebce4da` | 1054 | 1054 | `exact_hit` | `fnv64:2249cfc489e572a7` | `fnv64:2249cfc489e572a7` | 266 | 2.334393 | 2.335254 | 542240768 | 543682560 | 543682560 |

Every scenario reported:

```text
long_chat_result_finish_reason=length
long_chat_result_hit_token_limit=true
long_chat_result_streaming_content_chunks=1024
long_chat_result_streaming_token_id_chunks=1024
long_chat_result_streaming_token_ids=1024
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
long_chat_summary_generated_context_identity_required=true
long_chat_summary_generated_context_identity_links=3
long_chat_summary_matching_generated_context_identity_links=3
long_chat_summary_all_generated_context_identity_links_present=true
long_chat_summary_all_generated_context_identities_match_previous_response=true
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

This x86_64 run confirms the generated-context fixed-point mechanism with the
identity-summary gate enabled:

- turn 1 response hash equals turn 2 assistant-context hash;
- turn 2 response hash equals turn 3 assistant-context hash;
- turn 3 response hash equals turn 4 assistant-context hash;
- turn 3 response hash also equals its own assistant-context hash, so this lane
  reached a generated-response fixed point;
- turn 4 reused the full prompt: `1054` cached prompt tokens out of `1054`,
  `lookup=exact_hit`, and prompt hash equaled selected-entry hash.

TTFT followed cache depth:

- turn 2: `12 / 1054` cached prompt tokens, TTFT `382172` ms;
- turn 3: `16 / 1054` cached prompt tokens, TTFT `371694` ms;
- turn 4: `1054 / 1054` cached prompt tokens, TTFT `266` ms.

Decode throughput stayed much narrower than TTFT. Generated follow-up decode
throughput ranged from `2.286223` to `2.334393` tok/s.

RSS stayed bounded for this single run. Server idle RSS moved from
`457961472` bytes after turn 1 to `543682560` bytes after turn 4. The pod
cgroup peak after build and proof was `1462358016` bytes. This is single-run
bounded RSS evidence, not a leak-freedom claim.

## Operational Notes

The first detached server launch wrote the server process but did not write its
PID file because of shell grouping in the launch command. The running server
was healthy, the PID was recovered with `pgrep`, and the gate was launched
against that server PID for RSS sampling. This was an operator command issue,
not a Ferrite runtime failure.

The Kubernetes control plane briefly returned connection-refused,
`etcdserver: leader changed`, and one kubelet proxy `502 Bad Gateway` during
polling. Retries returned `ok` from `/readyz`, the pod had zero restarts, and
the gate exit files were both `0`.

## Limits

This run does not prove:

- 256/512-token x86 identity-summary behavior;
- stop/EOS-specific long-chat behavior;
- multi-run steady-state memory behavior;
- high-concurrency serving;
- release completeness for the long-chat milestone.
