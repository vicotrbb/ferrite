# Benchmark: X86 Qwen 1.5B Q8 Capsule Queue Proof 256

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Prove the capsule-aware generated-context identity fix on the staging x86_64
path with Qwen2.5-1.5B-Instruct Q8_0. The run uses Ferrite's
OpenAI-compatible streaming server, two prompt-cache keys, the queue probe,
error and disconnect reconnect probes, RSS sampling, token IDs, and a 256-token
4-turn matrix per key.

## Environment

- Ferrite commit: `52914b8`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-capsule-queue-qwen15-q8`
- Node: `homelab-01`
- Pod image: `rust:1.96-bookworm`
- Pod architecture: `x86_64`
- Rust: `rustc 1.96.0 (ac68faa20 2026-05-25)`
- Server PID for RSS sampling: `1607`
- Model: `Qwen2.5-1.5B-Instruct-Q8_0`
- Served model id: `qwen2.5-1.5b-instruct-q8_0`
- Model path in pod: `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
- Proof directory:
  `target/proof/x86-qwen-1-5b-q8-capsule-queue-proof-256-2026-07-03/`
- Server binary SHA256:
  `1d6ddc489e7baba1019dca0e947e64b29fb86874964e05b8542a66097b717686`
- Long-chat gate binary SHA256:
  `3a3d7bf63d2966b6cf8efb9a5b8b3fddf356159b72ff5ebbd7bdd4ee5c861d64`
- Model SHA256:
  `d7efb072e7724d25048a4fda0a3e10b04bdef5d06b1403a1c93bd9f1240a63c8`

The pod was deleted after copying artifacts back to local `target/proof/`.
`kubectl get pods -A | grep -i ferrite` returned no rows after cleanup.

## Server

```sh
RUST_LOG=info target/release/ferrite-server \
  --bind 127.0.0.1:18219 \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --model-id qwen2.5-1.5b-instruct-q8_0 \
  --api-key local-secret \
  --default-max-tokens 16 \
  --hard-max-tokens 256 \
  --inference-wait-ms 180000 \
  --experimental-prefix-cache
```

Readiness:

```text
GET /health -> {"status":"ok","ready":true,"model":"qwen2.5-1.5b-instruct-q8_0"}
```

## Gate

```sh
target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --queue-probe \
  --addr 127.0.0.1:18219 \
  --api-key local-secret \
  --models qwen2.5-1.5b-instruct-q8_0 \
  --token-lengths 256 \
  --turns 4 \
  --prompt "Summarize practical CPU inference engineering constraints in three compact points." \
  --assistant-context "Ferrite evaluates local OpenAI-compatible streaming chat under bounded CPU memory, prompt-cache behavior, reconnect behavior, and token latency evidence." \
  --follow-up "Continue with one additional compact engineering note and preserve the same structure." \
  --prompt-cache-keys ferrite:qwen15:q8:capsule-queue:a:256:2026-07-03,ferrite:qwen15:q8:capsule-queue:b:256:2026-07-03 \
  --prompt-cache-trace \
  --probe-max-tokens 64 \
  --generated-context-max-tokens 512 \
  --generated-context-state-capsule "State capsule: keep answers concise, number the points, and mention CPU, memory, and streaming reliability." \
  --generated-context-state-capsule-placement assistant-context \
  --disconnect-reconnect-timeout-ms 180000 \
  --rss-pid 1607 \
  --proof-log target/proof/x86-qwen-1-5b-q8-capsule-queue-proof-256-2026-07-03/long-chat-capsule-queue.log \
  --proof-exit-code target/proof/x86-qwen-1-5b-q8-capsule-queue-proof-256-2026-07-03/long-chat-capsule-queue.exit
```

The command exited `0`. The Kubernetes API reset/refused connections several
times while polling, but the gate process kept running inside the pod and
completed.

## Artifacts

| Artifact | Lines / bytes | SHA256 |
| --- | ---: | --- |
| `target/proof/x86-qwen-1-5b-q8-capsule-queue-proof-256-2026-07-03/long-chat-capsule-queue.log` | 397 lines | `79c252f20d6561025965323370787e6b8e9e37bfa218d524f8418ccbc77c6a8c` |
| `target/proof/x86-qwen-1-5b-q8-capsule-queue-proof-256-2026-07-03/long-chat-capsule-queue.exit` | 2 bytes | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |
| `target/proof/x86-qwen-1-5b-q8-capsule-queue-proof-256-2026-07-03/server.log` | 13 lines | `853f843e2ba23a6fa8c8bfe489a6a3fa950c84fb57201792018f6a8cab1a5e34` |
| `target/proof/x86-qwen-1-5b-q8-capsule-queue-proof-256-2026-07-03/server.stdout` | 0 bytes | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `target/proof/x86-qwen-1-5b-q8-capsule-queue-proof-256-2026-07-03/cgroup-memory-after.txt` | 2 lines | `6db7fdd8152897558032d446db868fb660ffcb3862c533e9c112f90cf8b622fb` |

Final cgroup samples:

```text
memory.current=3785756672
memory.peak=3918819328
```

## Queue Probe

```text
long_chat_queue_probe_holder_prompt_cache_key=ferrite:qwen15:q8:capsule-queue:a:256:2026-07-03
long_chat_queue_probe_contender_prompt_cache_key=ferrite:qwen15:q8:capsule-queue:b:256:2026-07-03
long_chat_queue_probe_holder_started_streaming=true
long_chat_queue_probe_holder_completed=true
long_chat_queue_probe_contender_status=200
long_chat_queue_probe_contender_completed=true
long_chat_queue_probe_contender_generated_event=true
long_chat_queue_probe_contender_started_after_holder=true
long_chat_queue_probe_max_tokens=64
```

## Scenario Results

| Lane | Turn | Prompt | Cached | Cache lookup | Shared prefix | Generated context hash | Generated response hash | TTFT ms | Decode tok/s | RSS before | RSS idle |
| --- | ---: | ---: | ---: | --- | ---: | --- | --- | ---: | ---: | ---: | ---: |
| A | 1 | 61 | 61 | exact_hit | 61 | n/a | `fnv64:d73a1059e11efa7d` | 89 | 4.069058 | 1941823488 | 1953619968 |
| A | 2 | 327 | 19 | shared_prefix_hit | 19 | `fnv64:d73a1059e11efa7d` | `fnv64:d14638fcedbed9eb` | 76227 | 3.573115 | 1953619968 | 1989140480 |
| A | 3 | 326 | 47 | shared_prefix_hit | 47 | `fnv64:d14638fcedbed9eb` | `fnv64:00683147820c47de` | 69378 | 3.570567 | 1989140480 | 2010505216 |
| A | 4 | 327 | 48 | shared_prefix_hit | 48 | `fnv64:00683147820c47de` | `fnv64:dd6cceac940b5d93` | 77113 | 3.564692 | 2010505216 | 2029379584 |
| B | 1 | 61 | 61 | exact_hit | 61 | n/a | `fnv64:d73a1059e11efa7d` | 95 | 4.018026 | 2029379584 | 2016825344 |
| B | 2 | 327 | 19 | shared_prefix_hit | 19 | `fnv64:d73a1059e11efa7d` | `fnv64:d14638fcedbed9eb` | 77121 | 3.569560 | 2016825344 | 2035175424 |
| B | 3 | 326 | 47 | shared_prefix_hit | 47 | `fnv64:d14638fcedbed9eb` | `fnv64:00683147820c47de` | 73632 | 3.584090 | 2035175424 | 2035175424 |
| B | 4 | 327 | 48 | shared_prefix_hit | 48 | `fnv64:00683147820c47de` | `fnv64:dd6cceac940b5d93` | 69637 | 3.595104 | 2035175424 | 2035175424 |

Every scenario reported 256 content chunks, 256 token-id chunks, valid usage
accounting, and `long_chat_result_hit_token_limit=true`.

## Integrated Summary

```text
long_chat_summary_planned_scenarios=8
long_chat_summary_completed_scenarios=8
long_chat_summary_all_finish_reasons_present=true
long_chat_summary_all_usage_accounting_valid=true
long_chat_summary_all_token_limit_status_present=true
long_chat_summary_any_token_limit_hit=true
long_chat_summary_any_cached_prompt_tokens=true
long_chat_summary_generated_follow_up_turns=6
long_chat_summary_cached_generated_follow_up_turns=6
long_chat_summary_uncached_generated_follow_up_turns=0
long_chat_summary_all_generated_follow_up_turns_cached=true
long_chat_summary_all_follow_up_turns_use_generated_context=true
long_chat_summary_generated_context_identity_links=6
long_chat_summary_matching_generated_context_identity_links=6
long_chat_summary_all_generated_context_identity_links_present=true
long_chat_summary_all_generated_context_identities_match_previous_response=true
long_chat_summary_all_timing_present=true
long_chat_summary_all_streaming_token_id_summaries_present=true
long_chat_summary_all_streaming_content_chunks_have_token_ids=true
long_chat_summary_all_rss_present=true
long_chat_summary_error_probe_completed=true
long_chat_summary_error_probe_reconnect_started_new_generation=true
long_chat_summary_disconnect_probe_completed=true
long_chat_summary_disconnect_probe_reconnect_started_new_generation=true
long_chat_summary_queue_probe_completed=true
long_chat_summary_queue_probe_contender_started_after_holder=true
long_chat_summary_run_complete=true
```

## Interpretation

The capsule-aware generated-context identity fix is now proven on both local
Qwen2.5-0.5B Q4_K_M and staging x86_64 Qwen2.5-1.5B Q8_0. The full rendered
assistant context can include a state capsule wrapper while
`long_chat_result_generated_context_hash` still proves continuity against the
previous generated response hash.

The Qwen2.5-1.5B Q8 path remains prefill-dominated for generated follow-up
turns. First-turn TTFT stayed under 100 ms after queue-probe warmup, while
generated-context turns had roughly 69-77 second TTFT at 256 tokens.

RSS stayed under the 6 Gi pod limit. The per-process RSS samples rose from about
1.94 GB to about 2.04 GB idle, and the cgroup peak was about 3.92 GB.

## Limits

- This is a 256-token proof only, not 512 or 1024.
- The queue probe warms both cache keys, so lane B turn 1 is not cold-key
  isolation evidence.
- Kubernetes API instability interrupted polling, but not the in-pod gate run.
- The run proves the OpenAI-compatible proof path, not production admission
  policy for state capsules.
