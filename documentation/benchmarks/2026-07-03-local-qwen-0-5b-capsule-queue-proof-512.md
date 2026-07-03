# Benchmark: Local Qwen 0.5B Capsule Queue Proof 512

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Extend the capsule queue proof from 256 to 512 generated tokens on the local
Qwen2.5-0.5B Q4_K_M OpenAI-compatible server path. The run verifies two
prompt-cache-key lanes, the queue probe, error and disconnect reconnect probes,
RSS sampling, token IDs, generated-context identity, and per-token timing at the
larger local completion budget.

## Environment

- Ferrite commit: `7a21757`
- Host: local macOS workspace
- Server: `127.0.0.1:18220`
- Server PID for RSS sampling: `1230`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Served model id: `qwen2.5-0.5b-q4_k_m`
- Proof directory:
  `target/proof/local-qwen05-capsule-queue-proof-512-2026-07-03/`
- Server binary SHA256:
  `50c221c62302c644f0278c5c52ead73e68cc5247e7fe154ff3bf4702d3d6cb59`
- Long-chat gate binary SHA256:
  `14949b0f4808afe248fbdf5e5be7c20dd5011fba92c74f8c8c5084438228e2a4`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

The local server was stopped after the run. A final bind-specific listener
check returned no listener on `127.0.0.1:18220`.

## Server

```sh
RUST_LOG=info target/release/ferrite-server \
  --bind 127.0.0.1:18220 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id qwen2.5-0.5b-q4_k_m \
  --api-key local-secret \
  --default-max-tokens 16 \
  --hard-max-tokens 512 \
  --inference-wait-ms 180000 \
  --experimental-prefix-cache
```

Readiness:

```text
GET /health -> {"status":"ok","ready":true,"model":"qwen2.5-0.5b-q4_k_m"}
```

## Gate

```sh
target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --queue-probe \
  --addr 127.0.0.1:18220 \
  --api-key local-secret \
  --models qwen2.5-0.5b-q4_k_m \
  --token-lengths 512 \
  --turns 4 \
  --prompt 'Summarize practical CPU inference engineering constraints in three compact points.' \
  --assistant-context 'Ferrite evaluates local OpenAI-compatible streaming chat under bounded CPU memory, prompt-cache behavior, reconnect behavior, and token latency evidence.' \
  --follow-up 'Continue with one additional compact engineering note and preserve the same structure.' \
  --prompt-cache-keys ferrite:qwen05:q4:capsule-queue:a:512:2026-07-03,ferrite:qwen05:q4:capsule-queue:b:512:2026-07-03 \
  --prompt-cache-trace \
  --probe-max-tokens 64 \
  --generated-context-max-tokens 512 \
  --generated-context-state-capsule 'State capsule: keep answers concise, number the points, and mention CPU, memory, and streaming reliability.' \
  --generated-context-state-capsule-placement assistant-context \
  --disconnect-reconnect-timeout-ms 180000 \
  --rss-pid 1230 \
  --proof-log target/proof/local-qwen05-capsule-queue-proof-512-2026-07-03/long-chat-capsule-queue.log \
  --proof-exit-code target/proof/local-qwen05-capsule-queue-proof-512-2026-07-03/long-chat-capsule-queue.exit
```

The command exited `0`.

## Artifacts

| Artifact | Lines / bytes | SHA256 |
| --- | ---: | --- |
| `target/proof/local-qwen05-capsule-queue-proof-512-2026-07-03/long-chat-capsule-queue.log` | 397 lines | `f59dd7c7e78e044f0b8da2a76854ec71fceb2f8bcf92e45ea54dcc8a756034ad` |
| `target/proof/local-qwen05-capsule-queue-proof-512-2026-07-03/long-chat-capsule-queue.exit` | 2 bytes | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |
| `target/proof/local-qwen05-capsule-queue-proof-512-2026-07-03/server.log` | 13 lines | `0cba8ae161e0f9ccf23411d9669593d055c13588a31a92586dc3860d5334f91e` |
| `target/proof/local-qwen05-capsule-queue-proof-512-2026-07-03/server.stdout` | 0 bytes | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

## Queue Probe

```text
long_chat_queue_probe_holder_prompt_cache_key=ferrite:qwen05:q4:capsule-queue:a:512:2026-07-03
long_chat_queue_probe_contender_prompt_cache_key=ferrite:qwen05:q4:capsule-queue:b:512:2026-07-03
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
| A | 1 | 61 | 61 | exact_hit | 61 | n/a | `fnv64:096eef8f230bc40a` | 72 | 19.026374 | 450723840 | 419463168 |
| A | 2 | 578 | 19 | shared_prefix_hit | 19 | `fnv64:096eef8f230bc40a` | `fnv64:87c2c31497c4392a` | 29627 | 12.541262 | 419463168 | 414023680 |
| A | 3 | 578 | 49 | shared_prefix_hit | 49 | `fnv64:87c2c31497c4392a` | `fnv64:1294b10c3711ddaf` | 28052 | 12.545556 | 414023680 | 410337280 |
| A | 4 | 578 | 47 | shared_prefix_hit | 47 | `fnv64:1294b10c3711ddaf` | `fnv64:1012e4db5db545b4` | 27866 | 12.567705 | 410337280 | 414187520 |
| B | 1 | 61 | 61 | exact_hit | 61 | n/a | `fnv64:096eef8f230bc40a` | 87 | 19.062908 | 414187520 | 430784512 |
| B | 2 | 578 | 19 | shared_prefix_hit | 19 | `fnv64:096eef8f230bc40a` | `fnv64:87c2c31497c4392a` | 29361 | 12.641571 | 430784512 | 421740544 |
| B | 3 | 578 | 49 | shared_prefix_hit | 49 | `fnv64:87c2c31497c4392a` | `fnv64:1294b10c3711ddaf` | 27987 | 12.661746 | 421740544 | 416432128 |
| B | 4 | 578 | 47 | shared_prefix_hit | 47 | `fnv64:1294b10c3711ddaf` | `fnv64:1012e4db5db545b4` | 28338 | 12.630484 | 416432128 | 421740544 |

Every scenario reported 512 content chunks, 512 token-id chunks, valid usage
accounting, and `long_chat_result_hit_token_limit=true`.

## Integrated Summary

```text
long_chat_summary_planned_scenarios=8
long_chat_summary_completed_scenarios=8
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

The local capsule queue proof now covers both 256 and 512 generated-token
budgets. The 512 run preserves the same generated-context identity behavior:
the state capsule changes the full assistant-context hash, while
`long_chat_result_generated_context_hash` continues to match the previous
generated-response hash across all six follow-up turns.

First-turn TTFT stayed under 100 ms after queue-probe warmup. Generated-context
turns had roughly 28-30 second TTFT, with decode throughput around
12.5-12.7 tok/s. RSS remained bounded around the same loaded-model range.

## Limits

- This is local Qwen2.5-0.5B Q4_K_M proof, not x86_64 Qwen2.5-1.5B Q8_0.
- The queue probe warms both cache keys, so lane B turn 1 is not cold-key
  namespace-isolation evidence.
- The 1024-token capsule queue proof remains open.
