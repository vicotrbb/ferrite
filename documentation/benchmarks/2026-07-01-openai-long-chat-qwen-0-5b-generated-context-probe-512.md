# OpenAI Long-Chat Qwen 0.5B Generated-Context 512-Token Probe

## Scope

This run extends the generated-context long-chat proof for the real local
`Qwen2.5-0.5B-Instruct-Q4_K_M` model from 256 to 512 completion tokens. It uses
the current OpenAI-compatible HTTP server path and the updated long-chat harness
that carries generated assistant text from each completed streaming response into
the next follow-up turn.

This is one local model and one additional token length. Together with the
256-token generated-context probe, it proves this model's generated follow-up
context shape at 256 and 512 tokens. It does not close the full Tier 1 long-chat
gate across the 1024-token generated-context length, larger artifacts, x86_64,
EOS-specific behavior, or steady-state serving.

## Environment

- Date: 2026-07-01
- Commit: `cddbfa5a2d42d1749dc2e8e1ba5d16df09effb67`
- Host: local macOS development machine
- Host architecture: `arm64`
- Build mode: release
- Server port: `127.0.0.1:18137`
- Server PID for RSS sampling: `36580`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Model path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`
- Server binary SHA256:
  `642f6b4cd2a521bd5bf1c7c5a78c78445c192189e09fb0fec538073a618a2fea`
- Long-chat gate binary SHA256:
  `ab79af5e2edaa975e4eafc51952ab27b842119d1a7056b3c767a195556ac360b`
- API key: `local-secret`
- Raw proof log:
  `target/proof/qwen-0-5b-long-chat-generated-context-probe-512.log`
- Raw proof exit file:
  `target/proof/qwen-0-5b-long-chat-generated-context-probe-512.exit`

The proof used a foreground server process held open by the tool session. After
the gate completed and the server was stopped, `lsof -nP -iTCP:18137
-sTCP:LISTEN` returned no listener.

## Server Command

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18137 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 512 \
  --hard-max-tokens 1024
```

Health check response:

```json
{"status":"ok","ready":true,"model":"Qwen2.5-0.5B-Instruct-Q4_K_M"}
```

## Gate Command

```sh
target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --models Qwen2.5-0.5B-Instruct-Q4_K_M \
  --token-lengths 512 \
  --turns 4 \
  --addr 127.0.0.1:18137 \
  --api-key local-secret \
  --rss-pid 36580 \
  --probe-max-tokens 512 \
  --expect-finish-reason length
```

The gate wrote `0` to
`target/proof/qwen-0-5b-long-chat-generated-context-probe-512.exit`.

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
| 1 | seed | 512 | 1 | length | 43 | 512 | 33915 | 513 | 1903 | 31901 | 16.080796 | 436486144 | 433373184 | 433356800 |
| 2 | generated | 512 | 1 | length | 542 | 512 | 106550 | 513 | 45941 | 104536 | 4.907364 | 433356800 | 410337280 | 410337280 |
| 3 | generated | 512 | 1 | length | 542 | 512 | 84219 | 513 | 31925 | 82206 | 6.240413 | 410337280 | 408879104 | 408879104 |
| 4 | generated | 512 | 1 | length | 542 | 512 | 84121 | 513 | 36681 | 82105 | 6.248051 | 408879104 | 410779648 | 410779648 |

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

The prompt-token count increased from `43` on the seed turn to `542` on
generated-context follow-up turns, showing the generated assistant output from
the prior completed stream is included in later requests.

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

Ferrite's OpenAI-compatible long-chat gate now has local Qwen2.5-0.5B Q4_K_M
generated-context proof at 256 and 512 completion tokens. The 512-token run
exercises larger carried assistant context than the 256-token run and still
records successful token-limit status, usage accounting, timing, RSS, error
recovery, disconnect recovery, fresh reconnect generation, and integrated
`long_chat_summary_run_complete=true`.

This remains partial evidence. The next proof work should repeat the
generated-context shape for 1024 tokens, then apply the same generated-context
matrix to the larger required Tier 1 HTTP model artifacts, x86_64, EOS-specific
long-chat behavior, and longer steady-state serving.
