# Benchmark: Local Qwen 0.5B Lifecycle Long-Chat Gate

Date: 2026-07-03 local time

## Purpose

Run the dedicated long-chat proof shape on the current tree after adding:

- OpenAI stream lifecycle server logs;
- generated-event validation for the long-chat error reconnect probe.

This run exercises the OpenAI-compatible streaming chat endpoint with Qwen2.5
0.5B Q4_K_M, 4 repeated turns, 256/512/1024 completion-token budgets, RSS
sampling, prompt-cache trace, request-error reconnect, client disconnect
reconnect, token-id coverage, generated-context identity checks, and
server-side lifecycle logs.

This is local arm64 evidence. It does not close x86_64, SmolLM2 1.7B, or full
Tier 1 multi-model coverage.

## Environment

- Ferrite commit: `48deae8`
- OS: Darwin `23.5.0`, arm64
- CPU: Apple M1 Pro
- Server bind: `127.0.0.1:18203`
- Server PID during run: `23122`
- Proof directory:
  `target/proof/local-qwen05-lifecycle-long-chat-2026-07-03/`

## Artifacts

- Long-chat proof log:
  `target/proof/local-qwen05-lifecycle-long-chat-2026-07-03/long-chat.log`
- Long-chat exit code:
  `target/proof/local-qwen05-lifecycle-long-chat-2026-07-03/long-chat.exit`
- Server lifecycle log:
  `target/proof/local-qwen05-lifecycle-long-chat-2026-07-03/server.log`

Artifact hashes:

```text
079c087e87d92f96dd717f5d83741f3a85ce7bc2a2612a9efa4fba516cb6e8db  target/proof/local-qwen05-lifecycle-long-chat-2026-07-03/long-chat.log
9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa  target/proof/local-qwen05-lifecycle-long-chat-2026-07-03/long-chat.exit
4fcb9b5feb8bb8e58b36fc55df8b8d9649000f70be03a9d7e65fdd0b3bbf9918  target/proof/local-qwen05-lifecycle-long-chat-2026-07-03/server.log
```

Binary and model hashes:

```text
9e6458f6ca175e830b253ef77e3d8205195f5597c3d6543ddc7c3e82f9061198  target/release/ferrite-server
92273a007b95a2f71d89cc69cf88dc66f11728e90279f112e89072aebd98de70  target/release/ferrite-openai-long-chat-gate
6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653  target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf
```

## Server Command

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18203 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id qwen2.5-0.5b-q4_k_m \
  --api-key local-secret \
  --default-max-tokens 1 \
  --hard-max-tokens 1024 \
  --inference-wait-ms 120000 \
  --experimental-prefix-cache
```

Readiness:

```text
{"status":"ok","ready":true,"model":"qwen2.5-0.5b-q4_k_m"}
```

## Gate Command

```sh
target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --addr 127.0.0.1:18203 \
  --api-key local-secret \
  --models qwen2.5-0.5b-q4_k_m \
  --token-lengths 256,512,1024 \
  --turns 4 \
  --prompt-cache-key local-qwen05-lifecycle-long-chat-2026-07-03 \
  --prompt-cache-trace \
  --expect-finish-reason length \
  --probe-max-tokens 256 \
  --disconnect-reconnect-timeout-ms 120000 \
  --rss-pid 23122 \
  --proof-log target/proof/local-qwen05-lifecycle-long-chat-2026-07-03/long-chat.log \
  --proof-exit-code target/proof/local-qwen05-lifecycle-long-chat-2026-07-03/long-chat.exit
```

The exit-code artifact contained `0`.

## Probe Results

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_error_probe_reconnect_generated_event=true
long_chat_error_probe_reconnect_started_new_generation=true
long_chat_error_probe_max_tokens=256
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
long_chat_disconnect_probe_reconnect_generated_event=true
long_chat_disconnect_probe_reconnect_started_new_generation=true
long_chat_disconnect_probe_max_tokens=256
```

The strengthened error probe now proves that the post-error reconnect generated
content instead of merely returning an SSE terminal event.

## Scenario Results

| Turn | Tokens | Source | Cache | Cached / Prompt | TTFT ms | Total ms | Stream tok/s | RSS idle bytes | Response hash |
| ---: | ---: | --- | --- | ---: | ---: | ---: | ---: | ---: | --- |
| 1 | 256 | seed | miss | 0 / 43 | 1660 | 13420 | 19.149855 | 425754624 | `fnv64:e13b6d98b69c8753` |
| 1 | 512 | seed | exact_hit | 43 / 43 | 75 | 26982 | 19.012375 | 426950656 | `fnv64:146530a2dc892984` |
| 1 | 1024 | seed | exact_hit | 43 / 43 | 76 | 67782 | 15.121910 | 408190976 | `fnv64:890bd91fd63ce8b0` |
| 2 | 256 | generated | shared_prefix_hit | 12 / 286 | 12159 | 27292 | 9.416599 | 428081152 | `fnv64:3c322262dcad4e06` |
| 2 | 512 | generated | shared_prefix_hit | 269 / 542 | 15905 | 55564 | 9.232490 | 420298752 | `fnv64:bb476bbb1a13c750` |
| 2 | 1024 | generated | shared_prefix_hit | 525 / 1054 | 40272 | 162456 | 6.309388 | 417087488 | `fnv64:d3b6392e4ebce4da` |
| 3 | 256 | generated | shared_prefix_hit | 14 / 286 | 12329 | 27277 | 9.421700 | 443416576 | `fnv64:4a28f15d57c5e5f2` |
| 3 | 512 | generated | shared_prefix_hit | 306 / 542 | 13616 | 53538 | 9.581924 | 445497344 | `fnv64:1a83e4a877b05975` |
| 3 | 1024 | generated | shared_prefix_hit | 16 / 1054 | 67224 | 191425 | 5.354565 | 420315136 | `fnv64:d3b6392e4ebce4da` |
| 4 | 256 | generated | shared_prefix_hit | 14 / 286 | 12327 | 27218 | 9.441951 | 438173696 | `fnv64:799ecadd9f0ad6b6` |
| 4 | 512 | generated | shared_prefix_hit | 20 / 542 | 26864 | 67502 | 7.599664 | 414203904 | `fnv64:2c24456c4fd5f9a7` |
| 4 | 1024 | generated | exact_hit | 1054 / 1054 | 231 | 118199 | 8.671745 | 416006144 | `fnv64:d3b6392e4ebce4da` |

## Summary Fields

```text
long_chat_summary_planned_scenarios=12
long_chat_summary_completed_scenarios=12
long_chat_summary_all_finish_reasons_present=true
long_chat_summary_all_usage_accounting_valid=true
long_chat_summary_all_token_limit_status_present=true
long_chat_summary_any_token_limit_hit=true
long_chat_summary_prompt_cache_key_present=true
long_chat_summary_any_cached_prompt_tokens=true
long_chat_summary_generated_follow_up_turns=9
long_chat_summary_cached_generated_follow_up_turns=9
long_chat_summary_uncached_generated_follow_up_turns=0
long_chat_summary_all_generated_follow_up_turns_cached=true
long_chat_summary_all_follow_up_turns_use_generated_context=true
long_chat_summary_generated_context_identity_links=9
long_chat_summary_matching_generated_context_identity_links=9
long_chat_summary_all_generated_context_identities_match_previous_response=true
long_chat_summary_all_timing_present=true
long_chat_summary_all_streaming_token_id_summaries_present=true
long_chat_summary_all_streaming_content_chunks_have_token_ids=true
long_chat_summary_all_rss_present=true
long_chat_summary_error_probe_completed=true
long_chat_summary_error_probe_reconnect_started_new_generation=true
long_chat_summary_disconnect_probe_completed=true
long_chat_summary_disconnect_probe_reconnect_started_new_generation=true
long_chat_summary_run_complete=true
```

## Lifecycle Log Summary

The server emitted 15 `openai_stream_lifecycle` lines:

- 14 completed streaming requests;
- 1 cancelled request from the disconnect probe;
- cancelled disconnect point: `token_streaming`;
- maximum request elapsed time: `191425 ms`;
- maximum prompt tokens started: `1038`;
- maximum prompt cancellation polls: `25950`;
- total generated token ids across lifecycle lines: `7682`.

The final exact-hit 1024 scenario produced:

```text
openai_stream_lifecycle request_id=stream-14 finish_reason=completed disconnect_point=none prompt_tokens_started=0 prompt_cancellation_polls=0 generated_chunks=1024 generated_token_ids=1024 elapsed_ms=118199
```

## Interpretation

This is a strong local proof for the Qwen 0.5B OpenAI-compatible long-chat
path:

- the 256/512/1024 token ladder completed for 4 turns;
- every scenario ended with `finish_reason=length`;
- usage accounting and token-limit status matched the requested token budgets;
- all streaming content chunks carried token IDs;
- RSS samples were present before, after, and idle for every scenario;
- all generated follow-up turns used generated assistant context;
- generated-context identity matched all 9 expected links;
- request-error and client-disconnect probes both reconnected into new
  generated streaming responses;
- lifecycle logs now provide server-side request elapsed time and prompt-work
  counters for the same run.

The 1024 lane reproduced the generated-context fixed-point pattern: turn 3
generated the same response identity that turn 4 used as assistant context, and
turn 4 became an exact cache hit (`1054 / 1054`) with TTFT dropping to `231 ms`.

## Limits

This run does not prove:

- x86_64 AVX2 behavior;
- SmolLM2 1.7B, Qwen 1.5B Q6_K, or Qwen 1.5B Q8_0 closure on the latest gate;
- EOS-specific terminal behavior;
- high-concurrency serving beyond the single-inference-permit design;
- `llama-benchy` companion parity for this exact run.

## Next Step

Use this artifact as the local-current baseline for the dedicated long-chat
gate. The next proof should repeat the same lifecycle-log shape on bounded
`staging` x86_64 hardware, then run the `llama-benchy` companion against the
same model/cache namespace if runtime budget allows.
