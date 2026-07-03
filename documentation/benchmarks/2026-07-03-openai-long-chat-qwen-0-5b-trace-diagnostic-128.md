# Benchmark: Qwen 0.5B Prompt-Cache Trace Diagnostic 128

Date: 2026-07-03 local time

## Purpose

Run a bounded OpenAI-compatible long-chat diagnostic against
`Qwen2.5-0.5B-Instruct-Q4_K_M` with the new prompt-cache trace enabled.

This run checks whether generated follow-up rows can now explain:

- cache lookup classification;
- prompt token hash;
- selected cache entry hash;
- shared-prefix depth;
- cached prompt tokens;
- TTFT and per-token latency;
- RSS before, after, and idle;
- stop-at-length behavior;
- unauthorized reconnect and client disconnect/reconnect behavior.

## Environment

- Ferrite commit: `f2dfbb4f1594f8d5240214351bd958e4672cbdbf`
- Host: local macOS development machine
- OS: Darwin arm64, `23.5.0`
- CPU: Apple M1 Pro
- Hardware threads: 8
- Memory: 17179869184 bytes
- Build mode: release
- Server: local Ferrite server on `127.0.0.1:18080`
- Server PID for RSS sampling: `24257`
- Raw proof log:
  `target/proof/qwen-0-5b-trace-diagnostic-128.log`
- Raw proof stdout:
  `target/proof/qwen-0-5b-trace-diagnostic-128.stdout`
- Raw server log:
  `target/proof/qwen-0-5b-trace-diagnostic-128-server.log`

## Model

- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Model path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- Served model id: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

## Binaries

- `target/release/ferrite-server` SHA256:
  `483dda38de9a0414f896c582617359325b880b64f001c76f60352979aea076ea`
- `target/release/ferrite-openai-long-chat-gate` SHA256:
  `a71141379a72b7b3db79b6357b20b8dcf614b44fa3f3ea75f4973bf644154c07`

## Build

```sh
cargo build -p ferrite-server --release --bins
```

Result:

```text
Finished `release` profile [optimized] target(s) in 6.09s
```

## Server

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18080 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 64 \
  --hard-max-tokens 128 \
  --inference-wait-ms 30000 \
  --experimental-prefix-cache
```

Readiness checks:

```text
GET /health -> {"status":"ok","ready":true,"model":"Qwen2.5-0.5B-Instruct-Q4_K_M"}
GET /v1/models -> {"object":"list","data":[{"id":"Qwen2.5-0.5B-Instruct-Q4_K_M","object":"model","created":0,"owned_by":"ferrite"}]}
```

## Gate

```sh
target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --addr 127.0.0.1:18080 \
  --api-key local-secret \
  --models Qwen2.5-0.5B-Instruct-Q4_K_M \
  --token-lengths 128 \
  --turns 4 \
  --probe-max-tokens 128 \
  --rss-pid 24257 \
  --prompt 'Write a compact note about generated-context prefix caching.' \
  --assistant-context 'Generated-context cache validation starts with a seed assistant message.' \
  --follow-up 'Continue the note and preserve the prior context.' \
  --prompt-cache-key ferrite:long-chat:qwen05:trace-diagnostic-128 \
  --prompt-cache-trace \
  --require-cached-follow-ups \
  --expect-finish-reason length \
  --proof-log target/proof/qwen-0-5b-trace-diagnostic-128.log \
  --proof-exit-code target/proof/qwen-0-5b-trace-diagnostic-128.exit
```

The durable exit-code file contained:

```text
0
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

| Turn | Context | Prompt | Cached | Lookup | Prompt hash | Selected entry hash | Shared prefix | TTFT ms | Decode tok/s | RSS before | RSS after | RSS idle |
| ---: | --- | ---: | ---: | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | seed | 39 | 0 | miss | `fnv64:abf4ec961f29675c` | | 0 | 2229 | 21.051273 | 437469184 | 422658048 | 422625280 |
| 2 | generated | 158 | 14 | shared_prefix_hit | `fnv64:f499cdb103f5cc99` | `fnv64:abf4ec961f29675c` | 14 | 6434 | 19.586387 | 422625280 | 440041472 | 440041472 |
| 3 | generated | 157 | 15 | shared_prefix_hit | `fnv64:f05569945319732c` | `fnv64:f499cdb103f5cc99` | 15 | 6338 | 19.586580 | 440041472 | 436207616 | 436207616 |
| 4 | generated | 158 | 16 | shared_prefix_hit | `fnv64:88b3b12c0a7e48fa` | `fnv64:f499cdb103f5cc99` | 16 | 6308 | 19.651973 | 436207616 | 447561728 | 445399040 |

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

The trace instrumentation works for this bounded real-model diagnostic. The
three generated follow-up turns all reported `shared_prefix_hit`, and
`cached_prompt_tokens` matched `shared_prefix_tokens` in each row. The selected
entry hash also explains which prior prompt was reused.

This is not a cache optimization proof. Reuse stayed shallow, at 14 to 16
tokens out of roughly 157 to 158 prompt tokens, and TTFT stayed around 6.3 to
6.4 seconds for generated follow-up turns. The diagnostic does prove that low
cache depth is now observable without reading generated text manually.

RSS stayed bounded for this short local run: measured idle RSS moved from
422625280 bytes after turn 1 to 445399040 bytes after turn 4. This is useful
diagnostic evidence, not leak-freedom proof.

## Limits

This run does not prove:

- x86_64 behavior for the new trace fields;
- 256, 512, or 1024-token traced behavior;
- full-prompt reuse stability;
- improved TTFT;
- long-running steady-state RSS behavior;
- stop/EOS traced behavior.

## Next Step

Run the same trace on the x86_64 1024-token lane, then compare each low-cache or
high-TTFT row against the selected entry hash and shared-prefix depth.
