# Benchmark: Qwen 0.5B Identity Summary Gate 512

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Run the OpenAI-compatible long-chat gate at the 512-token budget after adding
generated-context identity continuity to the run summary. This extends the local
identity-summary proof ladder from 256 tokens to 512 tokens.

This is a local 512-token proof slice. It is not the full 256/512/1024 matrix
and not x86_64 evidence.

## Environment

- Ferrite commit: `87b2533`
- Host: local macOS workspace
- Server bind: `127.0.0.1:18206`
- Server PID for RSS sampling: `47155`
- Raw artifacts:
  `target/proof/local-qwen05-identity-summary-512-2026-07-03/`

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
Finished `release` profile [optimized] target(s) in 0.22s
```

- `target/release/ferrite-server` SHA256:
  `17e4015060d188e61053fc53918ba7c97b827b0ee53f2b65cb8cce0ab101aee3`
- `target/release/ferrite-openai-long-chat-gate` SHA256:
  `9863c7b79c4fbf84d2079ff8f00c7305074802714419431960d2451d9981f384`

## Server

```sh
./target/release/ferrite-server \
  --bind 127.0.0.1:18206 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 512 \
  --hard-max-tokens 1024 \
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
  --addr 127.0.0.1:18206 \
  --api-key local-secret \
  --models Qwen2.5-0.5B-Instruct-Q4_K_M \
  --token-lengths 512 \
  --turns 4 \
  --probe-max-tokens 512 \
  --rss-pid 47155 \
  --prompt-cache-key ferrite:long-chat:qwen05:local-identity-summary-512 \
  --prompt-cache-trace \
  --require-cached-follow-ups \
  --expect-finish-reason length \
  --proof-log target/proof/local-qwen05-identity-summary-512-2026-07-03/local-qwen05-identity-summary-512.log \
  --proof-exit-code target/proof/local-qwen05-identity-summary-512-2026-07-03/local-qwen05-identity-summary-512.exit
```

Artifacts:

```text
local-qwen05-identity-summary-512.exit -> 0
210 target/proof/local-qwen05-identity-summary-512-2026-07-03/local-qwen05-identity-summary-512.log
210 target/proof/local-qwen05-identity-summary-512-2026-07-03/local-qwen05-identity-summary-512.stdout
```

## Probe Results

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_error_probe_max_tokens=512
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
long_chat_disconnect_probe_reconnect_generated_event=true
long_chat_disconnect_probe_reconnect_started_new_generation=true
long_chat_disconnect_probe_max_tokens=512
```

## Scenario Results

| Turn | Context source | Context bytes | Context hash | Response bytes | Response chunks | Response hash | Prompt tokens | Cached tokens | Lookup | Prompt hash | Selected entry hash | TTFT ms | Stream tok/s | RSS idle |
| ---: | --- | ---: | --- | ---: | ---: | --- | ---: | ---: | --- | --- | --- | ---: | ---: | ---: |
| 1 | seed | 97 | `fnv64:13669ce34c14a412` | 2663 | 512 | `fnv64:146530a2dc892984` | 43 | 0 | `miss` | `fnv64:92585af239e73208` | | 1787 | 17.098979 | 435552256 |
| 2 | generated | 2663 | `fnv64:146530a2dc892984` | 2685 | 512 | `fnv64:bb476bbb1a13c750` | 542 | 12 | `shared_prefix_hit` | `fnv64:adbd9eb91c7ffdfc` | `fnv64:92585af239e73208` | 28989 | 7.205546 | 412925952 |
| 3 | generated | 2685 | `fnv64:bb476bbb1a13c750` | 2839 | 512 | `fnv64:1a83e4a877b05975` | 542 | 306 | `shared_prefix_hit` | `fnv64:497487fe03f604b6` | `fnv64:adbd9eb91c7ffdfc` | 14369 | 9.181985 | 418889728 |
| 4 | generated | 2839 | `fnv64:1a83e4a877b05975` | 2820 | 512 | `fnv64:2c24456c4fd5f9a7` | 542 | 20 | `shared_prefix_hit` | `fnv64:af804d882bff135f` | `fnv64:497487fe03f604b6` | 28346 | 7.244283 | 417775616 |

Every scenario reported:

```text
long_chat_result_finish_reason=length
long_chat_result_hit_token_limit=true
long_chat_result_streaming_content_chunks=512
long_chat_result_streaming_token_id_chunks=512
long_chat_result_streaming_token_ids=512
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

The identity summary gate proves the generated-context carry-forward chain at
the 512-token budget:

- turn 1 response hash equals turn 2 assistant-context hash;
- turn 2 response hash equals turn 3 assistant-context hash;
- turn 3 response hash equals turn 4 assistant-context hash.

The run did not show a generated-response fixed point. Every generated response
hash changed. Cache depth was also unstable: turn 2 reused only 12 of 542 prompt
tokens, turn 3 reused 306 of 542, and turn 4 dropped back to 20 of 542. All
generated follow-up turns reported `shared_prefix_hit`, not `exact_hit`.

TTFT tracked cache depth directionally in this run: the high-reuse turn 3 had
lower TTFT (`14369` ms) than the shallow-reuse turns 2 and 4 (`28989` ms and
`28346` ms). Decode/stream throughput stayed much narrower than TTFT, with
generated follow-up stream throughput between `7.205546` and `9.181985` tok/s.

RSS stayed bounded in this short local run. Idle RSS moved from `435552256`
bytes after turn 1 to `417775616` bytes after turn 4. This is not leak-freedom
evidence.

## Limits

This run does not prove:

- 1024-token identity-summary behavior;
- x86_64 behavior;
- stop/EOS-specific long-chat behavior;
- steady-state memory behavior;
- high-concurrency serving;
- release completeness for the long-chat milestone.
