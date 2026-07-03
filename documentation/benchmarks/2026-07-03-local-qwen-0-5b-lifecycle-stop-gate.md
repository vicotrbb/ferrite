# Benchmark: Local Qwen 0.5B Lifecycle Stop Gate

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Refresh the explicit stop-sequence long-chat proof on the current
lifecycle-instrumented Ferrite server.

This proof targets stop behavior, not generated-context continuity. It verifies
that an explicit OpenAI `stop` sequence can produce `finish_reason=stop`, valid
usage accounting, per-token latency fields, RSS before/after samples,
error-probe reconnect behavior, disconnect-probe reconnect behavior, and a
successful integrated summary.

## Environment

- Ferrite commit: `3e74d37`
- Host: local macOS workspace
- Server: `127.0.0.1:18207`
- Server PID for RSS sampling: `32875`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Served model id: `qwen2.5-0.5b-q4_k_m`
- Proof directory:
  `target/proof/local-qwen05-lifecycle-stop-gate-fixed-2026-07-03/`
- Server binary SHA256:
  `9e6458f6ca175e830b253ef77e3d8205195f5597c3d6543ddc7c3e82f9061198`
- Long-chat gate binary SHA256:
  `414541d1efc8a64c12c8b26c2a3364d89cd54cca243e0e050496046d370eb8fa`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

The local server was stopped after the run. A final bind-specific process check
returned no listener on `127.0.0.1:18207`.

## Server

```sh
RUST_LOG=info target/release/ferrite-server \
  --bind 127.0.0.1:18207 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id qwen2.5-0.5b-q4_k_m \
  --api-key local-secret \
  --default-max-tokens 16 \
  --hard-max-tokens 256 \
  --inference-wait-ms 120000 \
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
  --addr 127.0.0.1:18207 \
  --api-key local-secret \
  --models qwen2.5-0.5b-q4_k_m \
  --token-lengths 1 \
  --turns 4 \
  --prompt 'hello world' \
  --assistant-context 'short context' \
  --follow-up 'hello world' \
  --stop '1' \
  --expect-finish-reason stop \
  --probe-max-tokens 32 \
  --disconnect-reconnect-timeout-ms 120000 \
  --rss-pid 32875 \
  --proof-log target/proof/local-qwen05-lifecycle-stop-gate-fixed-2026-07-03/long-chat-stop.log \
  --proof-exit-code target/proof/local-qwen05-lifecycle-stop-gate-fixed-2026-07-03/long-chat-stop.exit
```

The command exited `0`.

## Artifacts

| Artifact | Lines / bytes | SHA256 |
| --- | ---: | --- |
| `target/proof/local-qwen05-lifecycle-stop-gate-fixed-2026-07-03/long-chat-stop.log` | 170 lines | `8183051107aedca1d280ac6dfa3a227d938899da5bb0bc4bfb896bccdb0b088d` |
| `target/proof/local-qwen05-lifecycle-stop-gate-fixed-2026-07-03/long-chat-stop.exit` | 2 bytes | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |
| `target/proof/local-qwen05-lifecycle-stop-gate-fixed-2026-07-03/server.log` | 7 lines | `3dd84d4a74a873fb2724736c5bbdf92e24769c9eafc7d7b5e56418abd9dcce49` |
| `target/proof/local-qwen05-lifecycle-stop-gate-fixed-2026-07-03/server.stdout` | 0 bytes | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

## Probe Results

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_error_probe_reconnect_generated_event=true
long_chat_error_probe_reconnect_started_new_generation=true
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
long_chat_disconnect_probe_reconnect_generated_event=true
long_chat_disconnect_probe_reconnect_started_new_generation=true
```

## Scenario Results

| Turn | Finish | Hit token limit | TTFT ms | Token latency p50 ms | RSS before | RSS after | RSS idle |
| ---: | --- | --- | ---: | ---: | ---: | ---: | ---: |
| 1 | stop | false | 66 | 66 | 451133440 | 455360512 | 455344128 |
| 2 | stop | false | 75 | 75 | 455344128 | 455000064 | 455000064 |
| 3 | stop | false | 78 | 78 | 455000064 | 460619776 | 460619776 |
| 4 | stop | false | 77 | 77 | 460619776 | 461242368 | 461242368 |

Every scenario reported:

```text
long_chat_result_usage_prompt_tokens=18
long_chat_result_usage_cached_prompt_tokens=18
long_chat_result_usage_completion_tokens=1
long_chat_result_usage_total_tokens=19
```

## Integrated Summary

```text
long_chat_summary_planned_scenarios=4
long_chat_summary_completed_scenarios=4
long_chat_summary_all_finish_reasons_present=true
long_chat_summary_all_usage_accounting_valid=true
long_chat_summary_all_token_limit_status_present=true
long_chat_summary_any_token_limit_hit=false
long_chat_summary_generated_follow_up_context_required=false
long_chat_summary_all_follow_up_turns_use_generated_context=false
long_chat_summary_all_timing_present=true
long_chat_summary_streaming_token_ids_required=false
long_chat_summary_rss_required=true
long_chat_summary_all_rss_present=true
long_chat_summary_error_probe_completed=true
long_chat_summary_error_probe_reconnect_started_new_generation=true
long_chat_summary_disconnect_probe_completed=true
long_chat_summary_disconnect_probe_reconnect_started_new_generation=true
long_chat_summary_run_complete=true
```

## Server Lifecycle

The server emitted seven lifecycle lines:

- one completed error-probe reconnect stream with 32 generated token IDs;
- one cancelled disconnect-probe stream with `disconnect_point=token_streaming`;
- one completed disconnect-probe reconnect stream with 32 generated token IDs;
- four completed explicit-stop scenario streams with one generated token ID
  each.

No scenario stream reported a disconnect.

## Interpretation

This is current-commit proof that Ferrite's OpenAI-compatible streaming chat
path handles an explicit stop sequence through the integrated long-chat gate.
It also verifies that explicit-stop proof runs are not incorrectly failed for
lack of generated follow-up context after the stop filter removes visible
generated text.

This run does not prove natural tokenizer EOS behavior, larger Tier 1 models,
x86_64 behavior, concurrency, or 256/512/1024-token length behavior. Those
remain separate proof slices.
