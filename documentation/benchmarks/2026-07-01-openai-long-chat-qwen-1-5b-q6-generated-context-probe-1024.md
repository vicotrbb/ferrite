# OpenAI Long-Chat Qwen 1.5B Q6 Generated-Context 1024-Token Probe

## Scope

This run closes the local Q6_K larger-artifact generated-context token-length
set for the real local `Qwen2.5-1.5B-Instruct-Q6_K` model by extending the
proof from 256 and 512 to 1024 completion-token streaming chat responses. It
uses Ferrite's OpenAI-compatible HTTP server path and the long-chat harness
that carries generated assistant text from each completed streaming response
into the next follow-up turn.

This is one larger Tier 1 artifact and one token length. It proves the
generated assistant-context shape for Qwen2.5-1.5B Q6_K at 1024 completion
tokens, but it does not prove SmolLM2-1.7B, x86_64 generated-context behavior,
EOS-specific behavior, or steady-state serving.

## Environment

- Date: 2026-07-01
- Commit: `59b09d8a278375f8f18f9c5a6502349874b5891c`
- Host: local macOS development machine
- Host architecture: `arm64`
- Build mode: release
- Server port: `127.0.0.1:18144`
- Server PID for RSS sampling: `77733`
- Model: `Qwen2.5-1.5B-Instruct-Q6_K`
- Model path: `target/models/qwen2.5-1.5b-instruct-q6_k.gguf`
- Model SHA256:
  `e16d94f3b1eb243f6f6be9eee51090ef5dfd741324394fd5b6e0e425c33df5c7`
- Server binary SHA256:
  `642f6b4cd2a521bd5bf1c7c5a78c78445c192189e09fb0fec538073a618a2fea`
- Long-chat gate binary SHA256:
  `ab79af5e2edaa975e4eafc51952ab27b842119d1a7056b3c767a195556ac360b`
- API key: `local-secret`
- Raw proof log:
  `target/proof/qwen-1-5b-q6-long-chat-generated-context-probe-1024.log`
- Raw proof exit file:
  `target/proof/qwen-1-5b-q6-long-chat-generated-context-probe-1024.exit`

The proof used a foreground server process held open by the tool session. After
the gate completed and the server was stopped, `lsof -nP -iTCP:18144
-sTCP:LISTEN` returned no listener.

## Server Command

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18144 \
  --model target/models/qwen2.5-1.5b-instruct-q6_k.gguf \
  --model-id Qwen2.5-1.5B-Instruct-Q6_K \
  --api-key local-secret \
  --default-max-tokens 1024 \
  --hard-max-tokens 2048
```

Health check response:

```json
{"status":"ok","ready":true,"model":"Qwen2.5-1.5B-Instruct-Q6_K"}
```

## Gate Command

```sh
target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --models Qwen2.5-1.5B-Instruct-Q6_K \
  --token-lengths 1024 \
  --turns 4 \
  --addr 127.0.0.1:18144 \
  --api-key local-secret \
  --rss-pid 77733 \
  --probe-max-tokens 1024 \
  --expect-finish-reason length
```

The gate wrote `0` to
`target/proof/qwen-1-5b-q6-long-chat-generated-context-probe-1024.exit`.

## Probe Results

Both reconnect/error probes completed with the configured 1024-token budget:

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

The disconnect reconnect response included generated stream content and started
a fresh generation rather than resuming stale state.

## Scenario Results

All four streaming chat scenarios completed with `finish_reason=length`, usage
accounting for 1024 completion tokens, token-limit status, generated-context
status, streaming timing, per-token latency summaries, and RSS samples.

| Turn | Context | Max tokens | Completed | Finish | Prompt tokens | Completion tokens | Total ms | Events | TTFT ms | Stream ms | Tok/s | RSS before | RSS after | RSS idle |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | seed | 1024 | 1 | length | 43 | 1024 | 437917 | 1025 | 12005 | 435896 | 2.351476 | 1562066944 | 1541767168 | 1541767168 |
| 2 | generated | 1024 | 1 | length | 1075 | 1024 | 996485 | 1025 | 439179 | 994463 | 1.030707 | 1541767168 | 1618706432 | 1618706432 |
| 3 | generated | 1024 | 1 | length | 1060 | 1024 | 844462 | 1025 | 356865 | 842444 | 1.216698 | 1618706432 | 1617346560 | 1617346560 |
| 4 | generated | 1024 | 1 | length | 1055 | 1024 | 839424 | 1025 | 355681 | 837402 | 1.224023 | 1617346560 | 1619820544 | 1619820544 |

Each turn reported:

```text
long_chat_result_hit_token_limit=true
```

The generated-context status progressed as intended:

```text
long_chat_result_assistant_context_source=seed
long_chat_result_assistant_context_source=generated
long_chat_result_assistant_context_source=generated
long_chat_result_assistant_context_source=generated
```

The prompt-token count increased from `43` on the seed turn to `1075` on the
first generated-context follow-up turn. Turns 3 and 4 reported `1060` and
`1055` prompt tokens while still using generated assistant context.

## Integrated Summary

```text
long_chat_summary_planned_scenarios=4
long_chat_summary_completed_scenarios=4
long_chat_summary_all_finish_reasons_present=true
long_chat_summary_all_usage_accounting_valid=true
long_chat_summary_all_token_limit_status_present=true
long_chat_summary_any_token_limit_hit=true
long_chat_summary_all_follow_up_turns_use_generated_context=true
long_chat_summary_all_timing_present=true
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

Ferrite's OpenAI-compatible long-chat gate now has Qwen2.5-1.5B Q6_K
generated-context proof at 256, 512, and 1024 completion tokens. The 1024-token
run records token-limit status, usage accounting, timing, RSS, error recovery,
disconnect recovery, fresh reconnect generation, and integrated
`long_chat_summary_run_complete=true`.

This remains partial evidence for the full long-chat milestone. The next proof
work should cover SmolLM2-1.7B, x86_64 generated-context runs, EOS-specific
long-chat behavior, and longer steady-state serving.
