# Benchmark: Qwen 0.5B Identity Summary Gate 256

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Run the OpenAI-compatible long-chat gate after adding generated-context identity
continuity to the run summary. This proves the 256-token local Qwen 0.5B
generated-context path no longer requires manual row-by-row inspection to verify
that each generated response becomes the next turn's assistant context.

This is a local 256-token proof slice. It is not the full 256/512/1024 matrix
and not x86_64 evidence.

## Environment

- Ferrite commit: `928cca6`
- Host: local macOS workspace
- Server bind: `127.0.0.1:18205`
- Server PID for RSS sampling: `45812`
- Raw artifacts:
  `target/proof/local-qwen05-identity-summary-256-2026-07-03/`

The local server was stopped after the run. A final bind-specific process check
returned no process.

## Model

- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

## Binaries

The binaries were rebuilt from the current tree:

```sh
cargo build -p ferrite-server --release --bins
```

Result:

```text
Finished `release` profile [optimized] target(s) in 5.95s
```

- `target/release/ferrite-server` SHA256:
  `17e4015060d188e61053fc53918ba7c97b827b0ee53f2b65cb8cce0ab101aee3`
- `target/release/ferrite-openai-long-chat-gate` SHA256:
  `9863c7b79c4fbf84d2079ff8f00c7305074802714419431960d2451d9981f384`

## Server

```sh
./target/release/ferrite-server \
  --bind 127.0.0.1:18205 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 256 \
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
  --addr 127.0.0.1:18205 \
  --api-key local-secret \
  --models Qwen2.5-0.5B-Instruct-Q4_K_M \
  --token-lengths 256 \
  --turns 4 \
  --probe-max-tokens 256 \
  --rss-pid 45812 \
  --prompt-cache-key ferrite:long-chat:qwen05:local-identity-summary-256 \
  --prompt-cache-trace \
  --require-cached-follow-ups \
  --expect-finish-reason length \
  --proof-log target/proof/local-qwen05-identity-summary-256-2026-07-03/local-qwen05-identity-summary-256.log \
  --proof-exit-code target/proof/local-qwen05-identity-summary-256-2026-07-03/local-qwen05-identity-summary-256.exit
```

Artifacts:

```text
local-qwen05-identity-summary-256.exit -> 0
210 target/proof/local-qwen05-identity-summary-256-2026-07-03/local-qwen05-identity-summary-256.log
210 target/proof/local-qwen05-identity-summary-256-2026-07-03/local-qwen05-identity-summary-256.stdout
```

## Probe Results

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_error_probe_max_tokens=256
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
long_chat_disconnect_probe_reconnect_generated_event=true
long_chat_disconnect_probe_reconnect_started_new_generation=true
long_chat_disconnect_probe_max_tokens=256
```

## Scenario Results

| Turn | Context source | Context bytes | Context hash | Response bytes | Response chunks | Response hash | Prompt tokens | Cached tokens | Lookup | Prompt hash | Selected entry hash | TTFT ms | Stream tok/s | RSS idle |
| ---: | --- | ---: | --- | ---: | ---: | --- | ---: | ---: | --- | --- | --- | ---: | ---: | ---: |
| 1 | seed | 97 | `fnv64:13669ce34c14a412` | 1363 | 256 | `fnv64:e13b6d98b69c8753` | 43 | 0 | `miss` | `fnv64:92585af239e73208` | | 1807 | 17.590576 | 426016768 |
| 2 | generated | 1363 | `fnv64:e13b6d98b69c8753` | 1366 | 256 | `fnv64:3c322262dcad4e06` | 286 | 12 | `shared_prefix_hit` | `fnv64:f108453010484c86` | `fnv64:92585af239e73208` | 12988 | 8.937118 | 439861248 |
| 3 | generated | 1366 | `fnv64:3c322262dcad4e06` | 1326 | 256 | `fnv64:4a28f15d57c5e5f2` | 286 | 14 | `shared_prefix_hit` | `fnv64:62262a5e834a383b` | `fnv64:f108453010484c86` | 12831 | 8.906360 | 415039488 |
| 4 | generated | 1326 | `fnv64:4a28f15d57c5e5f2` | 1368 | 256 | `fnv64:799ecadd9f0ad6b6` | 286 | 14 | `shared_prefix_hit` | `fnv64:03a2694feddfd71a` | `fnv64:62262a5e834a383b` | 13049 | 8.942755 | 436486144 |

Every scenario reported:

```text
long_chat_result_finish_reason=length
long_chat_result_hit_token_limit=true
long_chat_result_streaming_content_chunks=256
long_chat_result_streaming_token_id_chunks=256
long_chat_result_streaming_token_ids=256
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

The new identity summary gate proves the generated-context carry-forward chain
without manual text inspection:

- turn 1 response hash equals turn 2 assistant-context hash;
- turn 2 response hash equals turn 3 assistant-context hash;
- turn 3 response hash equals turn 4 assistant-context hash.

The run did not show a generated-response fixed point. Each generated response
hash changed. Prompt-cache reuse stayed shallow on generated follow-ups:
12, 14, and 14 cached prompt tokens out of 286 prompt tokens, all reported as
`shared_prefix_hit`.

TTFT increased from `1807` ms on the seed turn to roughly `12.8` to `13.0`
seconds on generated follow-up turns. Decode throughput stayed much narrower
than TTFT, with generated follow-up stream throughput around `8.9` tok/s.

RSS stayed bounded in this short local run. Idle RSS moved from `426016768`
bytes after turn 1 to `436486144` bytes after turn 4. This is not leak-freedom
evidence.

## Limits

This run does not prove:

- 512 or 1024-token identity-summary behavior;
- x86_64 behavior;
- stop/EOS-specific long-chat behavior;
- steady-state memory behavior;
- high-concurrency serving;
- release completeness for the long-chat milestone.
