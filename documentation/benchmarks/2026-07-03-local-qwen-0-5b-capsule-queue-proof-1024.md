# Benchmark: Local Qwen 0.5B Capsule Queue Proof 1024

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Extend the local capsule queue proof from 256 and 512 generated tokens to 1024
generated tokens on the Qwen2.5-0.5B Q4_K_M OpenAI-compatible server path. The
run verifies two prompt-cache-key lanes, the queue probe, error and disconnect
reconnect probes, RSS sampling, token IDs, generated-context identity, and
per-token timing at the current largest local completion budget.

This run used a 1024-token retained generated-context window. A previous
aborted 1024-token attempt used a 512-token retained window, which can preserve
recent continuity but cannot prove full generated-response identity for
1024-token outputs.

## Environment

- Ferrite commit: `a5b9f9d`
- Host: local macOS workspace
- Server: `127.0.0.1:18221`
- Server PID for RSS sampling: `3883`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Served model id: `qwen2.5-0.5b-q4_k_m`
- Proof directory:
  `target/proof/local-qwen05-capsule-queue-proof-1024-full-context-2026-07-03/`
- Server binary SHA256:
  `50c221c62302c644f0278c5c52ead73e68cc5247e7fe154ff3bf4702d3d6cb59`
- Long-chat gate binary SHA256:
  `14949b0f4808afe248fbdf5e5be7c20dd5011fba92c74f8c8c5084438228e2a4`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

The local server was stopped after the run. A final bind-specific listener
check returned no listener on `127.0.0.1:18221`.

## Server

```sh
RUST_LOG=info target/release/ferrite-server \
  --bind 127.0.0.1:18221 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id qwen2.5-0.5b-q4_k_m \
  --api-key local-secret \
  --default-max-tokens 16 \
  --hard-max-tokens 1024 \
  --inference-wait-ms 240000 \
  --experimental-prefix-cache
```

## Gate

```sh
target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --queue-probe \
  --addr 127.0.0.1:18221 \
  --api-key local-secret \
  --models qwen2.5-0.5b-q4_k_m \
  --token-lengths 1024 \
  --turns 4 \
  --prompt 'Summarize practical CPU inference engineering constraints in three compact points.' \
  --assistant-context 'Ferrite evaluates local OpenAI-compatible streaming chat under bounded CPU memory, prompt-cache behavior, reconnect behavior, and token latency evidence.' \
  --follow-up 'Continue with one additional compact engineering note and preserve the same structure.' \
  --prompt-cache-keys ferrite:qwen05:q4:capsule-queue:a:1024-full:2026-07-03,ferrite:qwen05:q4:capsule-queue:b:1024-full:2026-07-03 \
  --prompt-cache-trace \
  --probe-max-tokens 64 \
  --generated-context-max-tokens 1024 \
  --generated-context-state-capsule 'State capsule: keep answers concise, number the points, and mention CPU, memory, and streaming reliability.' \
  --generated-context-state-capsule-placement assistant-context \
  --disconnect-reconnect-timeout-ms 240000 \
  --rss-pid 3883 \
  --proof-log target/proof/local-qwen05-capsule-queue-proof-1024-full-context-2026-07-03/long-chat-capsule-queue.log \
  --proof-exit-code target/proof/local-qwen05-capsule-queue-proof-1024-full-context-2026-07-03/long-chat-capsule-queue.exit
```

The command exited `0`.

## Artifacts

| Artifact | Lines / bytes | SHA256 |
| --- | ---: | --- |
| `target/proof/local-qwen05-capsule-queue-proof-1024-full-context-2026-07-03/long-chat-capsule-queue.log` | 396 lines | `9e5c5df6eb837e69c937203b67967927863111082764ba3046c4619123bd033f` |
| `target/proof/local-qwen05-capsule-queue-proof-1024-full-context-2026-07-03/long-chat-capsule-queue.exit` | 2 bytes | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |

## Queue Probe

```text
long_chat_queue_probe_holder_prompt_cache_key=ferrite:qwen05:q4:capsule-queue:a:1024-full:2026-07-03
long_chat_queue_probe_contender_prompt_cache_key=ferrite:qwen05:q4:capsule-queue:b:1024-full:2026-07-03
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
| A | 1 | 61 | 61 | exact_hit | 61 | n/a | `fnv64:6142d1e1666afbb9` | 72 | 15.080941 | 445759488 | 409862144 |
| A | 2 | 1090 | 19 | shared_prefix_hit | 19 | `fnv64:6142d1e1666afbb9` | `fnv64:9c1f91c12a5552f0` | 70280 | 8.104634 | 409862144 | 420085760 |
| A | 3 | 1089 | 47 | shared_prefix_hit | 47 | `fnv64:9c1f91c12a5552f0` | `fnv64:f48ba93daa92144c` | 68503 | 8.214243 | 420085760 | 421560320 |
| A | 4 | 1090 | 47 | shared_prefix_hit | 47 | `fnv64:f48ba93daa92144c` | `fnv64:611b3422b269a9a3` | 68750 | 8.227020 | 421560320 | 430080000 |
| B | 1 | 61 | 0 | miss | 0 | n/a | `fnv64:6142d1e1666afbb9` | 2349 | 15.051316 | 430080000 | 418873344 |
| B | 2 | 1090 | 19 | shared_prefix_hit | 19 | `fnv64:6142d1e1666afbb9` | `fnv64:9c1f91c12a5552f0` | 69855 | 8.161672 | 418873344 | 418185216 |
| B | 3 | 1089 | 47 | shared_prefix_hit | 47 | `fnv64:9c1f91c12a5552f0` | `fnv64:f48ba93daa92144c` | 67899 | 8.191236 | 418185216 | 414973952 |
| B | 4 | 1090 | 47 | shared_prefix_hit | 47 | `fnv64:f48ba93daa92144c` | `fnv64:611b3422b269a9a3` | 68616 | 8.185511 | 414973952 | 418660352 |

Every scenario reported 1024 content chunks, 1024 token-id chunks, valid usage
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

The local capsule queue proof now covers 256, 512, and 1024 generated-token
budgets. The 1024 run preserves full generated-context identity across all six
follow-up turns when the retained generated-context window is also 1024 tokens.

Generated-context turns took roughly 68-70 second TTFT on this local CPU path,
with decode throughput around 8.1-8.2 tok/s. RSS stayed within the loaded-model
range, with sampled idle RSS between roughly 410 MB and 430 MB.

Lane B turn 1 is useful stronger namespace evidence in this run because the
queue probe did not leave that key as an exact prompt-cache hit; the first B
scenario reported `long_chat_result_prompt_cache_lookup=miss` and
`long_chat_result_usage_cached_prompt_tokens=0`, while follow-up turns under
the same key still reported shared-prefix hits.

## Limits

- This is local Qwen2.5-0.5B Q4_K_M proof, not x86_64 Qwen2.5-1.5B Q8_0.
- The run proves full generated-context identity only because
  `--generated-context-max-tokens` matched the 1024-token completion budget.
- x86_64 512-token and 1024-token capsule queue proofs remain open.
