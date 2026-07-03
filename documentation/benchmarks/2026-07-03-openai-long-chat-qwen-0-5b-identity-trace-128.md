# Benchmark: Qwen 0.5B Response Identity Trace 128

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Run a bounded local OpenAI-compatible long-chat gate after adding
response/context identity output. This checks whether the proof tooling can
compare a generated assistant response with the next turn's assistant context
without printing generated text.

This is a local instrumentation proof, not an x86_64 performance proof.

## Environment

- Ferrite commit: `0930ee3`
- Host: local macOS workspace
- Server bind: `127.0.0.1:18203`
- Server PID for RSS sampling: `42780`
- Raw artifacts:
  `target/proof/local-qwen05-identity-trace-128-2026-07-03/`

The local server was stopped after artifact collection. A final process check
for the bind-specific command returned no process.

## Model

- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

## Binaries

- `target/release/ferrite-server` SHA256:
  `3fd89b31ff30a89ae3e0a999b2db8ca8e96d2f36afe844b5d495a216a97de19e`
- `target/release/ferrite-openai-long-chat-gate` SHA256:
  `eca7fbf78bf3a63fc33f80279c00b2562714fa9675f8f0da2377d832c8ed2bd8`

Build command:

```sh
cargo build -p ferrite-server --release --bins
```

Result:

```text
Finished `release` profile [optimized] target(s) in 5.83s
```

## Server

```sh
./target/release/ferrite-server \
  --bind 127.0.0.1:18203 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 128 \
  --hard-max-tokens 512 \
  --inference-wait-ms 30000 \
  --experimental-prefix-cache
```

Readiness:

```text
GET /health -> {"status":"ok","ready":true,"model":"Qwen2.5-0.5B-Instruct-Q4_K_M"}
```

## Gate

```sh
./target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --addr 127.0.0.1:18203 \
  --api-key local-secret \
  --models Qwen2.5-0.5B-Instruct-Q4_K_M \
  --token-lengths 128 \
  --turns 4 \
  --probe-max-tokens 128 \
  --rss-pid 42780 \
  --prompt-cache-key ferrite:long-chat:qwen05:local-identity-trace-128 \
  --prompt-cache-trace \
  --require-cached-follow-ups \
  --expect-finish-reason length \
  --proof-log target/proof/local-qwen05-identity-trace-128-2026-07-03/local-qwen05-identity-trace-128.log \
  --proof-exit-code target/proof/local-qwen05-identity-trace-128-2026-07-03/local-qwen05-identity-trace-128.exit
```

Artifacts:

```text
local-qwen05-identity-trace-128.exit -> 0
205 target/proof/local-qwen05-identity-trace-128-2026-07-03/local-qwen05-identity-trace-128.log
205 target/proof/local-qwen05-identity-trace-128-2026-07-03/local-qwen05-identity-trace-128.stdout
```

## Probe Results

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_error_probe_max_tokens=128
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
long_chat_disconnect_probe_reconnect_generated_event=true
long_chat_disconnect_probe_reconnect_started_new_generation=true
long_chat_disconnect_probe_max_tokens=128
```

## Scenario Results

| Turn | Context source | Context bytes | Context hash | Response bytes | Response chunks | Response hash | Prompt tokens | Cached tokens | Lookup | Prompt hash | Selected entry hash | TTFT ms | RSS idle |
| ---: | --- | ---: | --- | ---: | ---: | --- | ---: | ---: | --- | --- | --- | ---: | ---: |
| 1 | seed | 97 | `fnv64:13669ce34c14a412` | 664 | 128 | `fnv64:d6e2f2c865e49919` | 43 | 0 | `miss` | `fnv64:92585af239e73208` | | 1855 | 425541632 |
| 2 | generated | 664 | `fnv64:d6e2f2c865e49919` | 781 | 128 | `fnv64:0969ba966218802c` | 158 | 12 | `shared_prefix_hit` | `fnv64:190167027293ebca` | `fnv64:92585af239e73208` | 6439 | 437354496 |
| 3 | generated | 781 | `fnv64:0969ba966218802c` | 674 | 128 | `fnv64:a449d2a8d7a2519c` | 158 | 13 | `shared_prefix_hit` | `fnv64:c5e8918d59c0909c` | `fnv64:190167027293ebca` | 6445 | 436649984 |
| 4 | generated | 674 | `fnv64:a449d2a8d7a2519c` | 689 | 128 | `fnv64:3f00097c7fd454d6` | 158 | 14 | `shared_prefix_hit` | `fnv64:94c9b8b70f8aa4a6` | `fnv64:190167027293ebca` | 6384 | 436371456 |

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

The new identity output proves the generated-context carry-forward path:

- turn 1 response hash equals turn 2 assistant-context hash;
- turn 2 response hash equals turn 3 assistant-context hash;
- turn 3 response hash equals turn 4 assistant-context hash.

This run does not show a generated-response fixed point. Each generated
response hash differs from the previous response hash. Prompt-cache reuse stays
shallow at 12, 13, and 14 cached prompt tokens on generated follow-up turns,
and each generated turn reports `shared_prefix_hit` rather than `exact_hit`.

The selected-entry trace also shows that turn 4 selected the turn 2 prompt hash,
not the immediately previous turn 3 prompt hash. That suggests this local
128-token path is not the same cache fixed-point behavior observed in the x86
1024-token trace.

RSS stayed bounded in this short local run. Idle RSS moved from `425541632`
bytes after turn 1 to `436371456` bytes after turn 4. This is not leak-freedom
evidence.

## Limits

This run does not prove:

- x86_64 behavior;
- 256, 512, or 1024-token identity traces;
- steady-state memory behavior;
- EOS/stop-specific generated-context behavior;
- an optimization effect from response identity tracing.
