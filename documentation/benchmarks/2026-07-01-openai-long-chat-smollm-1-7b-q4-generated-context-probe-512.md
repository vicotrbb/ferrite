# OpenAI Long-Chat SmolLM2 1.7B Q4 Generated-Context 512-Token Probe

## Scope

This run extends the SmolLM2-1.7B Q4_K_M generated-context proof set for the
OpenAI-compatible long-chat gate from 256 to 512 completion tokens. It uses the
current generated assistant carry-forward harness: turn 1 starts with the
configured seed assistant context, and turns 2-4 use assistant text generated
by prior completed streaming responses.

This is one local model and one token length. It proves the generated-context
shape for SmolLM2-1.7B Q4_K_M at 512 completion tokens, but it does not prove
the 1024-token SmolLM2 generated-context length, x86_64 generated-context
behavior, broader EOS behavior, leak freedom, or longer steady-state serving.

The run uses the same full-length operational prompt shape as the earlier
SmolLM2 long-chat proof set because the harness default prompt can terminate
early with `finish_reason=stop` for this model.

## Environment

- Date: 2026-07-01
- Commit: `dc9b2b100b2a3107bf6e1b69b36af69a2895dfda`
- Host: local macOS development machine
- Host architecture: `arm64`
- Build mode: release
- Server port: `127.0.0.1:18146`
- Server PID for RSS sampling: `68860`
- Model: `SmolLM2-1.7B-Instruct-Q4_K_M`
- Model path: `target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf`
- Model SHA256:
  `77665ea4815999596525c636fbeb56ba8b080b46ae85efef4f0d986a139834d7`
- Server binary SHA256:
  `642f6b4cd2a521bd5bf1c7c5a78c78445c192189e09fb0fec538073a618a2fea`
- Long-chat gate binary SHA256:
  `ab79af5e2edaa975e4eafc51952ab27b842119d1a7056b3c767a195556ac360b`
- API key: `local-secret`
- Raw proof log:
  `target/proof/smollm-1-7b-q4-long-chat-generated-context-probe-512.log`
- Raw proof exit file:
  `target/proof/smollm-1-7b-q4-long-chat-generated-context-probe-512.exit`

The proof used a foreground server process held open by the tool session. After
the gate completed and the server was stopped, `lsof -nP -iTCP:18146
-sTCP:LISTEN` returned no listener.

## Server Command

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18146 \
  --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf \
  --model-id SmolLM2-1.7B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 512 \
  --hard-max-tokens 1024
```

Health check response:

```json
{"status":"ok","ready":true,"model":"SmolLM2-1.7B-Instruct-Q4_K_M"}
```

## Gate Command

```sh
target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --models SmolLM2-1.7B-Instruct-Q4_K_M \
  --token-lengths 512 \
  --turns 4 \
  --addr 127.0.0.1:18146 \
  --api-key local-secret \
  --rss-pid 68860 \
  --probe-max-tokens 512 \
  --expect-finish-reason length \
  --prompt "Write a concise operational note about CPU inference stability." \
  --assistant-context "CPU inference stability depends on bounded memory use, predictable token latency, and clear server health signals." \
  --follow-up "Continue with reconnect and error-handling risks."
```

The gate wrote `0` to
`target/proof/smollm-1-7b-q4-long-chat-generated-context-probe-512.exit`.

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
| 1 | seed | 512 | 1 | length | 53 | 512 | 127881 | 513 | 9406 | 125865 | 4.075765 | 1231257600 | 1250508800 | 1250508800 |
| 2 | generated | 512 | 1 | length | 546 | 512 | 315136 | 513 | 121650 | 313121 | 1.638340 | 1250508800 | 1303085056 | 1303085056 |
| 3 | generated | 512 | 1 | length | 546 | 512 | 314078 | 513 | 120822 | 312063 | 1.643897 | 1303085056 | 1351991296 | 1351991296 |
| 4 | generated | 512 | 1 | length | 546 | 512 | 318276 | 513 | 124002 | 316261 | 1.622076 | 1351991296 | 1389936640 | 1389936640 |

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

The prompt-token count increased from `53` on the seed turn to `546` on
generated-context follow-up turns, showing the larger carried assistant context
was included.

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

Ferrite's OpenAI-compatible long-chat gate now has SmolLM2-1.7B Q4_K_M
generated-context proof at 256 and 512 completion tokens. The 512-token run
records token-limit status, generated assistant-context carry-forward, usage
accounting, timing, RSS, error recovery, disconnect recovery, fresh reconnect
generation, and integrated `long_chat_summary_run_complete=true`.

This remains partial evidence for the full long-chat milestone. The next proof
work should cover SmolLM2-1.7B generated-context 1024-token behavior, x86_64
generated-context runs, EOS-specific long-chat behavior, and longer
steady-state serving.
