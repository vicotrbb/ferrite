# OpenAI Long-Chat Qwen 1.5B Q8 Generated-Context 512-Token Probe

## Scope

This run extends the larger-artifact generated-context proof set for the real
local `Qwen2.5-1.5B-Instruct-Q8_0` model from 256 to 512 completion-token
streaming chat responses. It uses Ferrite's OpenAI-compatible HTTP server path
and the long-chat harness that carries generated assistant text from each
completed streaming response into the next follow-up turn.

This is one larger Tier 1 artifact and one token length. It proves the
generated assistant-context shape for Qwen2.5-1.5B Q8_0 at 512 completion
tokens, but it does not prove this model's 1024 generated-context length,
Qwen2.5-1.5B Q6_K, SmolLM2-1.7B, x86_64 generated-context behavior,
EOS-specific behavior, or steady-state serving.

## Environment

- Date: 2026-07-01
- Commit: `599840bf4657636b4e6689a31b98d14de6699758`
- Host: local macOS development machine
- Host architecture: `arm64`
- Build mode: release
- Server port: `127.0.0.1:18140`
- Server PID for RSS sampling: `77785`
- Model: `Qwen2.5-1.5B-Instruct-Q8_0`
- Model path: `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
- Model SHA256:
  `d7efb072e7724d25048a4fda0a3e10b04bdef5d06b1403a1c93bd9f1240a63c8`
- Server binary SHA256:
  `642f6b4cd2a521bd5bf1c7c5a78c78445c192189e09fb0fec538073a618a2fea`
- Long-chat gate binary SHA256:
  `ab79af5e2edaa975e4eafc51952ab27b842119d1a7056b3c767a195556ac360b`
- API key: `local-secret`
- Raw proof log:
  `target/proof/qwen-1-5b-q8-long-chat-generated-context-probe-512.log`
- Raw proof exit file:
  `target/proof/qwen-1-5b-q8-long-chat-generated-context-probe-512.exit`

The proof used a foreground server process held open by the tool session. After
the gate completed and the server was stopped, `lsof -nP -iTCP:18140
-sTCP:LISTEN` returned no listener.

## Server Command

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18140 \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --model-id Qwen2.5-1.5B-Instruct-Q8_0 \
  --api-key local-secret \
  --default-max-tokens 512 \
  --hard-max-tokens 1024
```

Health check response:

```json
{"status":"ok","ready":true,"model":"Qwen2.5-1.5B-Instruct-Q8_0"}
```

## Gate Command

```sh
target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --models Qwen2.5-1.5B-Instruct-Q8_0 \
  --token-lengths 512 \
  --turns 4 \
  --addr 127.0.0.1:18140 \
  --api-key local-secret \
  --rss-pid 77785 \
  --probe-max-tokens 512 \
  --expect-finish-reason length
```

The gate wrote `0` to
`target/proof/qwen-1-5b-q8-long-chat-generated-context-probe-512.exit`.

## Probe Results

Both reconnect/error probes completed with the configured 512-token budget:

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

The disconnect reconnect response included generated stream content and started
a fresh generation rather than resuming stale state.

## Scenario Results

All four streaming chat scenarios completed with `finish_reason=length`, usage
accounting for 512 completion tokens, token-limit status, generated-context
status, streaming timing, per-token latency summaries, and RSS samples.

| Turn | Context | Max tokens | Completed | Finish | Prompt tokens | Completion tokens | Total ms | Events | TTFT ms | Stream ms | Tok/s | RSS before | RSS after | RSS idle |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | seed | 512 | 1 | length | 43 | 512 | 96522 | 513 | 4201 | 94508 | 5.428107 | 1688043520 | 1683603456 | 1682374656 |
| 2 | generated | 512 | 1 | length | 553 | 512 | 190900 | 513 | 76473 | 188886 | 2.715923 | 1682374656 | 1706033152 | 1706016768 |
| 3 | generated | 512 | 1 | length | 543 | 512 | 377972 | 513 | 142303 | 375956 | 1.364521 | 1706016768 | 1698365440 | 1698365440 |
| 4 | generated | 512 | 1 | length | 533 | 512 | 366256 | 513 | 133540 | 364198 | 1.408571 | 1698365440 | 1688731648 | 366837760 |

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

The prompt-token count increased from `43` on the seed turn to `553` on the
first generated-context follow-up turn. Turns 3 and 4 reported `543` and `533`
prompt tokens while still using generated assistant context.

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

Ferrite's OpenAI-compatible long-chat gate now has larger-artifact
generated-context proof for Qwen2.5-1.5B Q8_0 at 256 and 512 completion tokens.
The 512-token run records token-limit status, usage accounting, timing, RSS,
error recovery, disconnect recovery, fresh reconnect generation, and integrated
`long_chat_summary_run_complete=true`.

This remains partial evidence. The next proof work should repeat this
generated-context shape for Qwen2.5-1.5B Q8_0 at 1024 completion tokens, then
cover Qwen2.5-1.5B Q6_K, SmolLM2-1.7B, x86_64 generated-context runs,
EOS-specific long-chat behavior, and longer steady-state serving.
