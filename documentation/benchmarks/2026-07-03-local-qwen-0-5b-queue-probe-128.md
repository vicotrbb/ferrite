# Benchmark: Local Qwen 0.5B Queue Probe 128

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Exercise the OpenAI long-chat queue probe with a bounded 128-token local
Qwen2.5-0.5B run. This proves the queue probe path can be required and
completed, but it is not a replacement for the 256/512/1024 long-chat matrix.

## Environment

- Ferrite runtime code commit: `8c1cc4f`
- Host: local macOS workspace
- Server: `127.0.0.1:18234`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Served model id: `qwen2.5-0.5b-q4_k_m`
- Proof directory: `target/proof/local-qwen05-queue-probe-128-2026-07-03/`
- Server binary SHA256:
  `dec0167a646244de6392efbfe5b1549c4064dbab729de894aaa87c02c988b473`
- Gate binary SHA256:
  `7a953e710de9210b2832d61fa55dc89a8f835d5207a7e18659d9f9480ab03e97`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

The local server was stopped after the run. A final bind-specific listener
check returned no listener on `127.0.0.1:18234`.

## Server

```sh
RUST_LOG=info target/release/ferrite-server \
  --bind 127.0.0.1:18234 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id qwen2.5-0.5b-q4_k_m \
  --api-key local-secret \
  --default-max-tokens 256 \
  --hard-max-tokens 512 \
  --inference-wait-ms 120000 \
  --experimental-prefix-cache
```

Readiness:

```text
GET /health -> {"status":"ok","ready":true,"model":"qwen2.5-0.5b-q4_k_m"}
```

## Gate Command

```sh
target/release/ferrite-openai-long-chat-gate \
  --addr 127.0.0.1:18234 \
  --api-key local-secret \
  --execute \
  --models qwen2.5-0.5b-q4_k_m \
  --require-models qwen2.5-0.5b-q4_k_m \
  --token-lengths 128 \
  --require-token-lengths 128 \
  --turns 4 \
  --rss-pid <server-pid> \
  --queue-probe \
  --require-probes queue \
  --probe-max-tokens 64 \
  --prompt-cache-keys queue-a,queue-b \
  --prompt-cache-trace \
  --proof-log target/proof/local-qwen05-queue-probe-128-2026-07-03/long-chat.log \
  --proof-exit-code target/proof/local-qwen05-queue-probe-128-2026-07-03/long-chat.exit
```

The queue probe requires at least two prompt-cache keys. This accepted run used
`queue-a` for the holder lane and `queue-b` for the contender lane.

## Artifacts

| Artifact | Lines | SHA256 |
| --- | ---: | --- |
| `target/proof/local-qwen05-queue-probe-128-2026-07-03/long-chat.log` | 390 | `2e058bb943b6be19ecda895ccbe26d3c1ae3253808f4a9f41d5debb30148bf73` |
| `target/proof/local-qwen05-queue-probe-128-2026-07-03/gate.stdout` | 390 | `2e058bb943b6be19ecda895ccbe26d3c1ae3253808f4a9f41d5debb30148bf73` |
| `target/proof/local-qwen05-queue-probe-128-2026-07-03/gate.stderr` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `target/proof/local-qwen05-queue-probe-128-2026-07-03/server.log` | 10 | `55a86a29f80f25e1754de7ed80d55304305287741aa6b04e68d2d67ac88a1498` |
| `target/proof/local-qwen05-queue-probe-128-2026-07-03/server.stdout` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `target/proof/local-qwen05-queue-probe-128-2026-07-03/health.json` | 0 | `e3284eada962df1c75177574e65d3c528a2dcc0fb990143e5877c096413857b4` |
| `target/proof/local-qwen05-queue-probe-128-2026-07-03/long-chat.exit` | 1 | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |
| `target/proof/local-qwen05-queue-probe-128-2026-07-03/gate-command.exit` | 1 | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |

Both exit-code files contained `0`. The health file contains one JSON payload
without a trailing newline, so `wc -l` reports `0`.

## Queue Probe Results

```text
long_chat_queue_probe_required=true
long_chat_queue_probe_holder_prompt_cache_key=queue-a
long_chat_queue_probe_contender_prompt_cache_key=queue-b
long_chat_queue_probe_holder_started_streaming=true
long_chat_queue_probe_holder_completed=true
long_chat_queue_probe_contender_status=200
long_chat_queue_probe_contender_completed=true
long_chat_queue_probe_contender_generated_event=true
long_chat_queue_probe_contender_started_after_holder=true
long_chat_queue_probe_max_tokens=64
```

## Scenario Results

The run executed four 128-token turns for each prompt-cache key.

| Key | Turn | Finish | Completion tokens | TTFT ms | Tok/s | RSS before | RSS after | RSS idle |
| --- | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| queue-a | 1 | length | 128 | 3 | 24.616669 | 457441280 | 460079104 | 460079104 |
| queue-a | 2 | length | 128 | 5844 | 10.979935 | 460079104 | 467550208 | 467550208 |
| queue-a | 3 | length | 128 | 5813 | 10.897798 | 467550208 | 473792512 | 473792512 |
| queue-a | 4 | length | 128 | 5881 | 10.864389 | 473792512 | 476315648 | 476315648 |
| queue-b | 1 | length | 128 | 5 | 24.684312 | 476315648 | 477577216 | 477577216 |
| queue-b | 2 | length | 128 | 5898 | 10.859624 | 477577216 | 477626368 | 477626368 |
| queue-b | 3 | length | 128 | 5773 | 10.972535 | 477626368 | 482689024 | 482689024 |
| queue-b | 4 | length | 128 | 5728 | 11.007744 | 482689024 | 486965248 | 486965248 |

## Summary

```text
long_chat_summary_planned_scenarios=8
long_chat_summary_completed_scenarios=8
long_chat_summary_all_finish_reasons_present=true
long_chat_summary_all_usage_accounting_valid=true
long_chat_summary_all_token_limit_status_present=true
long_chat_summary_any_token_limit_hit=true
long_chat_summary_prompt_cache_key_present=true
long_chat_summary_all_follow_up_turns_use_generated_context=true
long_chat_summary_all_generated_context_identities_match_previous_response=true
long_chat_summary_all_timing_present=true
long_chat_summary_all_streaming_token_id_summaries_present=true
long_chat_summary_all_streaming_content_chunks_have_token_ids=true
long_chat_summary_rss_required=true
long_chat_summary_all_rss_present=true
long_chat_summary_queue_probe_required=true
long_chat_summary_queue_probe_completed=true
long_chat_summary_queue_probe_contender_started_after_holder=true
long_chat_summary_required_probes=queue
long_chat_summary_required_probes_completed=true
long_chat_summary_required_models=qwen2.5-0.5b-q4_k_m
long_chat_summary_required_models_present=true
long_chat_summary_required_token_lengths=128
long_chat_summary_required_token_lengths_present=true
long_chat_summary_run_complete=true
```

## Interpretation

This validates the queue probe path on a bounded local Qwen2.5-0.5B slice. The
holder stream started, the contender request returned `200`, the contender
generated an event, and the harness proved the contender started after the
holder.

This is not full Tier 1 queue closure. It uses one local model, a 128-token
budget, and only the queue probe. The full long-chat gate still needs queue
behavior included in the broader required model and token-length matrix.

## Next Step

Add a stop/EOS slice, then decide whether the full closure run should require
`error,disconnect,queue` together for every Tier 1 model and token length.
