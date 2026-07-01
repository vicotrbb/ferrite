# OpenAI Long-Chat Qwen 1.5B Q8 Generated-Context 256-Token Probe

## Scope

This run starts the larger-artifact generated-context proof set for the real
local `Qwen2.5-1.5B-Instruct-Q8_0` model at a 256 completion-token streaming
chat length. It uses Ferrite's OpenAI-compatible HTTP server path and the
long-chat harness that carries generated assistant text from each completed
streaming response into the next follow-up turn.

This is one larger Tier 1 artifact and one token length. It proves the
generated assistant-context shape for Qwen2.5-1.5B Q8_0 at 256 completion
tokens, but it does not prove this model's 512/1024 generated-context lengths,
Qwen2.5-1.5B Q6_K, SmolLM2-1.7B, x86_64 generated-context behavior,
EOS-specific behavior, or steady-state serving.

## Environment

- Date: 2026-07-01
- Commit: `20c9b8afbc8bba63a669f60f41101896d24a053b`
- Host: local macOS development machine
- Host architecture: `arm64`
- Build mode: release
- Server port: `127.0.0.1:18139`
- Server PID for RSS sampling: `64608`
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
  `target/proof/qwen-1-5b-q8-long-chat-generated-context-probe-256.log`
- Raw proof exit file:
  `target/proof/qwen-1-5b-q8-long-chat-generated-context-probe-256.exit`

The proof used a foreground server process held open by the tool session. After
the gate completed and the server was stopped, `lsof -nP -iTCP:18139
-sTCP:LISTEN` returned no listener.

## Server Command

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18139 \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --model-id Qwen2.5-1.5B-Instruct-Q8_0 \
  --api-key local-secret \
  --default-max-tokens 256 \
  --hard-max-tokens 512
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
  --token-lengths 256 \
  --turns 4 \
  --addr 127.0.0.1:18139 \
  --api-key local-secret \
  --rss-pid 64608 \
  --probe-max-tokens 256 \
  --expect-finish-reason length
```

The gate wrote `0` to
`target/proof/qwen-1-5b-q8-long-chat-generated-context-probe-256.exit`.

## Probe Results

Both reconnect/error probes completed with the configured 256-token budget:

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

The disconnect reconnect response included generated stream content and started
a fresh generation rather than resuming stale state.

## Scenario Results

All four streaming chat scenarios completed with `finish_reason=length`, usage
accounting for 256 completion tokens, token-limit status, generated-context
status, streaming timing, per-token latency summaries, and RSS samples.

| Turn | Context | Max tokens | Completed | Finish | Prompt tokens | Completion tokens | Total ms | Events | TTFT ms | Stream ms | Tok/s | RSS before | RSS after | RSS idle |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | seed | 256 | 1 | length | 43 | 256 | 102779 | 257 | 22059 | 100749 | 2.550892 | 1667874816 | 1667678208 | 1667678208 |
| 2 | generated | 256 | 1 | length | 287 | 256 | 83047 | 257 | 39900 | 81036 | 3.171419 | 1667678208 | 1684897792 | 1684897792 |
| 3 | generated | 256 | 1 | length | 287 | 256 | 70797 | 257 | 31363 | 68785 | 3.736265 | 1684897792 | 1689518080 | 1689042944 |
| 4 | generated | 256 | 1 | length | 282 | 256 | 72061 | 257 | 33489 | 70049 | 3.668851 | 1689042944 | 1701904384 | 1701904384 |

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

The prompt-token count increased from `43` on the seed turn to `287` on the
first two generated-context follow-up turns. Turn 4 reported `282` prompt
tokens while still using generated assistant context.

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

Ferrite's OpenAI-compatible long-chat gate now has one larger-artifact
generated-context proof: Qwen2.5-1.5B Q8_0 at 256 completion tokens. The run
records token-limit status, usage accounting, timing, RSS, error recovery,
disconnect recovery, fresh reconnect generation, and integrated
`long_chat_summary_run_complete=true`.

This remains partial evidence. The next proof work should repeat this
generated-context shape for Qwen2.5-1.5B Q8_0 at 512 and 1024 completion tokens,
then cover Qwen2.5-1.5B Q6_K, SmolLM2-1.7B, x86_64 generated-context runs,
EOS-specific long-chat behavior, and longer steady-state serving.
