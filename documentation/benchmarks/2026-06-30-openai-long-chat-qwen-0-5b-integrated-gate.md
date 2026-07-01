# OpenAI Long-Chat Qwen 0.5B Integrated Gate

## Scope

This run exercises the long-chat gate's integrated proof shape for
Qwen2.5 0.5B Q4_K_M. It combines the 256/512/1024-token streaming chat matrix,
four repeated turns, RSS sampling, latency summaries, request-level error
recovery, client disconnect recovery, and the new `long_chat_summary_*` output
in one invocation.

This is not a full Tier 1 long-chat closure. It is a single-model integrated
gate run that proves the combined report shape works on the fastest required
Tier 1 model.

## Environment

- Date: 2026-06-30
- Commit: `29b7ba0`
- Host: local macOS development machine
- Server port: `127.0.0.1:18099`
- Server PID for RSS sampling: `88874`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Model path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- API key: `local-secret`

## Server Command

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18099 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 256 \
  --hard-max-tokens 1024
```

Health check response:

```json
{"status":"ok","ready":true,"model":"Qwen2.5-0.5B-Instruct-Q4_K_M"}
```

## Gate Command

```sh
cargo run -p ferrite-server --bin ferrite-openai-long-chat-gate -- \
  --execute \
  --error-probe \
  --disconnect-probe \
  --models Qwen2.5-0.5B-Instruct-Q4_K_M \
  --token-lengths 256,512,1024 \
  --turns 4 \
  --addr 127.0.0.1:18099 \
  --api-key local-secret \
  --rss-pid 88874 \
  --prompt 'Write a concise operational note about CPU inference stability.' \
  --assistant-context 'CPU inference stability depends on bounded memory use, predictable token latency, and clear server health signals.' \
  --follow-up 'Continue with reconnect and error-handling risks.'
```

## Probe Results

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
```

The disconnect result records clean retry behavior, not resumable SSE streams.

## Matrix Results

All twelve streaming chat scenarios completed with `finish_reason=length`.
Usage completion tokens matched the requested token length for every scenario,
and streaming token event counts matched completion token counts.

| Turn | Max tokens | Completed | Finish | Total ms | Stream ms | Tok/s | TTFT ms | p50 ms | p95 ms | RSS before | RSS after | RSS idle |
| --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 256 | 1 | length | 19191 | 17103 | 14.967581 | 2241 | 56 | 89 | 425492480 | 408076288 | 408076288 |
| 1 | 512 | 1 | length | 45669 | 43554 | 11.755394 | 2533 | 75 | 133 | 408076288 | 408961024 | 408895488 |
| 1 | 1024 | 1 | length | 158050 | 155818 | 6.571748 | 17779 | 102 | 376 | 408895488 | 409387008 | 407404544 |
| 2 | 256 | 1 | length | 21309 | 19192 | 13.338530 | 2601 | 61 | 101 | 407404544 | 406290432 | 265207808 |
| 2 | 512 | 1 | length | 43730 | 41587 | 12.311359 | 2627 | 71 | 123 | 265207808 | 407683072 | 392855552 |
| 2 | 1024 | 1 | length | 82838 | 80657 | 12.695647 | 2656 | 76 | 100 | 392855552 | 409632768 | 409387008 |
| 3 | 256 | 1 | length | 17098 | 15011 | 17.053513 | 2019 | 49 | 61 | 409387008 | 422805504 | 422412288 |
| 3 | 512 | 1 | length | 34712 | 32577 | 15.716279 | 2045 | 58 | 79 | 422412288 | 408813568 | 408780800 |
| 3 | 1024 | 1 | length | 85316 | 83130 | 12.318025 | 2378 | 74 | 116 | 408780800 | 410730496 | 410730496 |
| 4 | 256 | 1 | length | 17270 | 15155 | 16.891488 | 2123 | 49 | 61 | 410730496 | 408027136 | 406257664 |
| 4 | 512 | 1 | length | 39868 | 37753 | 13.561754 | 2087 | 56 | 106 | 406241280 | 408584192 | 401539072 |
| 4 | 1024 | 1 | length | 84246 | 82052 | 12.479874 | 2086 | 73 | 111 | 401539072 | 409845760 | 409468928 |

Usage was stable by token length:

- `256`: prompt tokens `47`, completion tokens `256`, total tokens `303`.
- `512`: prompt tokens `47`, completion tokens `512`, total tokens `559`.
- `1024`: prompt tokens `47`, completion tokens `1024`, total tokens `1071`.

One idle RSS sample dropped to `265207808` bytes between turn 2 scenarios and
then returned to the expected range on the next request. The summary still
records all RSS samples as present; this is an observed sampler/runtime
discontinuity, not a missing sample.

## Integrated Summary

```text
long_chat_summary_planned_scenarios=12
long_chat_summary_completed_scenarios=12
long_chat_summary_all_finish_reasons_present=true
long_chat_summary_all_usage_accounting_valid=true
long_chat_summary_all_timing_present=true
long_chat_summary_rss_required=true
long_chat_summary_all_rss_present=true
long_chat_summary_error_probe_required=true
long_chat_summary_error_probe_completed=true
long_chat_summary_disconnect_probe_required=true
long_chat_summary_disconnect_probe_completed=true
long_chat_summary_run_complete=true
```

After stopping the server, `lsof -nP -iTCP:18099 -sTCP:LISTEN` returned no
listener.

## Interpretation

Ferrite completed a single-model integrated long-chat gate run through the
OpenAI-compatible HTTP server. The run proves the new summary output can bind
scenario completion, finish reasons, token accounting, timing, RSS presence,
error recovery, and disconnect recovery into one machine-readable conclusion.

Observed throughput:

- 256-token scenarios: about `13.34` to `17.05` tok/s.
- 512-token scenarios: about `11.76` to `15.72` tok/s.
- 1024-token scenarios: about `6.57` to `12.70` tok/s.

Remaining proof gaps:

- Repeat integrated error/disconnect summary runs for Qwen2.5 1.5B Q8_0,
  Qwen2.5 1.5B Q6_K, and SmolLM2 1.7B Q4_K_M.
- Combine explicit stop assertions with the integrated summary output.
- Add EOS-specific evidence once Ferrite exposes a distinct EOS terminal reason
  through the OpenAI-compatible stream.
