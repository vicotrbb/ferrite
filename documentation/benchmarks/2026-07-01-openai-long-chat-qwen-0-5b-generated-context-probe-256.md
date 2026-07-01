# OpenAI Long-Chat Qwen 0.5B Generated-Context 256-Token Probe

## Scope

This run verifies the updated long-chat gate against the real local
`Qwen2.5-0.5B-Instruct-Q4_K_M` model at a 256-token streaming chat length. It
exercises the generated-context carry-forward harness added after the earlier
fixed-context long-chat proofs.

This is one local model and one token length. It proves the new generated
assistant-context fields and stricter reconnect fields on a real OpenAI-compatible
HTTP server path, but it does not close the full Tier 1 long-chat gate across
512/1024-token lengths, larger artifacts, x86_64, EOS-specific behavior, or
steady-state serving.

## Environment

- Date: 2026-07-01
- Commit: `3591b844224a606a52f58d1972e58fb5a143e1cb`
- Host: local macOS development machine
- Host architecture: `arm64`
- Build mode: release
- Server port: `127.0.0.1:18136`
- Server PID for RSS sampling: `19907`
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
  `target/proof/qwen-0-5b-long-chat-generated-context-probe-256.log`
- Raw proof exit file:
  `target/proof/qwen-0-5b-long-chat-generated-context-probe-256.exit`

The proof used a foreground server process held open by the tool session. An
earlier wrapper attempt launched the server from a short-lived shell and then
lost the listener before the gate connected. The clean run below used the
foreground server and wrote exit code `0`.

## Server Command

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18136 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 256 \
  --hard-max-tokens 512
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
  --token-lengths 256 \
  --turns 4 \
  --addr 127.0.0.1:18136 \
  --api-key local-secret \
  --rss-pid 19907 \
  --probe-max-tokens 256 \
  --expect-finish-reason length
```

The gate wrote `0` to
`target/proof/qwen-0-5b-long-chat-generated-context-probe-256.exit`.

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

The disconnect reconnect response had to include generated stream content as
well as `[DONE]`; a done-only SSE response is no longer accepted by the harness.

## Scenario Results

All four streaming chat scenarios completed with `finish_reason=length`, usage
accounting for 256 completion tokens, token-limit status, generated-context
status, streaming timing, and RSS samples.

| Turn | Context | Max tokens | Completed | Finish | Prompt tokens | Completion tokens | Total ms | Events | TTFT ms | Stream ms | Tok/s | RSS before | RSS after | RSS idle |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | seed | 256 | 1 | length | 43 | 256 | 17338 | 257 | 1948 | 15323 | 16.771144 | 407093248 | 433537024 | 433537024 |
| 2 | generated | 256 | 1 | length | 286 | 256 | 34732 | 257 | 14790 | 32722 | 7.854043 | 433537024 | 435077120 | 435077120 |
| 3 | generated | 256 | 1 | length | 286 | 256 | 37002 | 257 | 14938 | 34991 | 7.344736 | 435077120 | 414810112 | 414810112 |
| 4 | generated | 256 | 1 | length | 286 | 256 | 38164 | 257 | 17953 | 36143 | 7.110644 | 414810112 | 408911872 | 408829952 |

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

After stopping the foreground server, `lsof -nP -iTCP:18136 -sTCP:LISTEN`
returned no listener.

## Interpretation

Ferrite's updated OpenAI-compatible long-chat gate now has one real-model proof
that follow-up turns use generated assistant context rather than a fixed seed
assistant message. The prompt-token count increases from `43` on the seed turn
to `286` on generated-context follow-up turns, showing the longer generated
assistant response is included in later requests.

This remains partial evidence. The next proof work should repeat the
generated-context shape for 512 and 1024 tokens, the other required Tier 1 HTTP
model artifacts, x86_64, EOS-specific long-chat behavior, and longer
steady-state serving.
