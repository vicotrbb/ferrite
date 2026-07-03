# Benchmark: Qwen 0.5B Identity Summary Gate 1024

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Run the OpenAI-compatible long-chat gate at the 1024-token budget after adding
generated-context identity continuity to the run summary. This completes the
local Qwen 0.5B identity-summary proof ladder for 256, 512, and 1024 completion
tokens.

This is local macOS evidence, not x86_64 evidence.

## Environment

- Ferrite commit: `6173cae`
- Host: local macOS workspace
- Server bind: `127.0.0.1:18207`
- Server PID for RSS sampling: `48963`
- Raw artifacts:
  `target/proof/local-qwen05-identity-summary-1024-2026-07-03/`

The local server was stopped after the run. A final bind-specific process check
returned no process.

## Model

- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

## Binaries

The binaries were built from the current tree:

```sh
cargo build -p ferrite-server --release --bins
```

Result:

```text
Finished `release` profile [optimized] target(s) in 0.18s
```

- `target/release/ferrite-server` SHA256:
  `17e4015060d188e61053fc53918ba7c97b827b0ee53f2b65cb8cce0ab101aee3`
- `target/release/ferrite-openai-long-chat-gate` SHA256:
  `9863c7b79c4fbf84d2079ff8f00c7305074802714419431960d2451d9981f384`

## Server

```sh
./target/release/ferrite-server \
  --bind 127.0.0.1:18207 \
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

## Gate

```sh
./target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --addr 127.0.0.1:18207 \
  --api-key local-secret \
  --models Qwen2.5-0.5B-Instruct-Q4_K_M \
  --token-lengths 1024 \
  --turns 4 \
  --probe-max-tokens 1024 \
  --rss-pid 48963 \
  --prompt-cache-key ferrite:long-chat:qwen05:local-identity-summary-1024 \
  --prompt-cache-trace \
  --require-cached-follow-ups \
  --expect-finish-reason length \
  --proof-log target/proof/local-qwen05-identity-summary-1024-2026-07-03/local-qwen05-identity-summary-1024.log \
  --proof-exit-code target/proof/local-qwen05-identity-summary-1024-2026-07-03/local-qwen05-identity-summary-1024.exit
```

Artifacts:

```text
local-qwen05-identity-summary-1024.exit -> 0
210 target/proof/local-qwen05-identity-summary-1024-2026-07-03/local-qwen05-identity-summary-1024.log
210 target/proof/local-qwen05-identity-summary-1024-2026-07-03/local-qwen05-identity-summary-1024.stdout
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

| Turn | Context source | Context bytes | Context hash | Response bytes | Response chunks | Response hash | Prompt tokens | Cached tokens | Lookup | Prompt hash | Selected entry hash | TTFT ms | Stream tok/s | RSS idle |
| ---: | --- | ---: | --- | ---: | ---: | --- | ---: | ---: | --- | --- | --- | ---: | ---: | ---: |
| 1 | seed | 97 | `fnv64:13669ce34c14a412` | 5274 | 1024 | `fnv64:890bd91fd63ce8b0` | 43 | 0 | `miss` | `fnv64:92585af239e73208` | | 1797 | 13.959322 | 409419776 |
| 2 | generated | 5274 | `fnv64:890bd91fd63ce8b0` | 5161 | 1024 | `fnv64:d3b6392e4ebce4da` | 1054 | 12 | `shared_prefix_hit` | `fnv64:93e2cf81835f98a6` | `fnv64:92585af239e73208` | 71818 | 5.076401 | 415449088 |
| 3 | generated | 5161 | `fnv64:d3b6392e4ebce4da` | 5161 | 1024 | `fnv64:d3b6392e4ebce4da` | 1054 | 16 | `shared_prefix_hit` | `fnv64:2249cfc489e572a7` | `fnv64:93e2cf81835f98a6` | 70904 | 5.113237 | 425197568 |
| 4 | generated | 5161 | `fnv64:d3b6392e4ebce4da` | 5161 | 1024 | `fnv64:d3b6392e4ebce4da` | 1054 | 1054 | `exact_hit` | `fnv64:2249cfc489e572a7` | `fnv64:2249cfc489e572a7` | 228 | 8.462663 | 420741120 |

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

This run locally confirms the generated-context fixed-point mechanism at the
1024-token budget:

- turn 1 response hash equals turn 2 assistant-context hash;
- turn 2 response hash equals turn 3 assistant-context hash;
- turn 3 response hash equals turn 4 assistant-context hash;
- turn 3 response hash also equals turn 3 assistant-context hash, showing the
  generated response had become a fixed point for that lane;
- turn 4 reused the full prompt: `1054` cached prompt tokens out of `1054`,
  `lookup=exact_hit`, and prompt hash equaled selected-entry hash.

TTFT followed cache depth strongly:

- turn 2: 12 / 1054 cached prompt tokens, TTFT `71818` ms;
- turn 3: 16 / 1054 cached prompt tokens, TTFT `70904` ms;
- turn 4: 1054 / 1054 cached prompt tokens, TTFT `228` ms.

Decode throughput stayed much narrower than TTFT. Generated follow-up stream
throughput ranged from `5.076401` to `8.462663` tok/s.

RSS stayed bounded in this short local run. Idle RSS moved from `409419776`
bytes after turn 1 to `420741120` bytes after turn 4. This is not leak-freedom
evidence.

## Limits

This run does not prove:

- x86_64 behavior;
- stop/EOS-specific long-chat behavior;
- steady-state memory behavior;
- high-concurrency serving;
- release completeness for the long-chat milestone.
