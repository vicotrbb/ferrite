# OpenAI Long-Chat Qwen 0.5B Generated-Context 1024-Token Probe

## Scope

This run completes the local generated-context token-length set for the real
`Qwen2.5-0.5B-Instruct-Q4_K_M` model by adding the 1024 completion-token
streaming chat length. It uses the current OpenAI-compatible HTTP server path
and the long-chat harness that carries generated assistant text from each
completed streaming response into the next follow-up turn.

Together with the 256-token and 512-token generated-context probes, this proves
the local Qwen2.5-0.5B Q4_K_M generated follow-up context shape at 256, 512, and
1024 completion tokens. It does not close the full Tier 1 long-chat gate across
larger artifacts, x86_64 generated-context reruns, EOS-specific behavior, or
steady-state serving.

## Environment

- Date: 2026-07-01
- Commit: `3eba26690fe6cf73819274d9abc748a3fe5232fb`
- Host: local macOS development machine
- Host architecture: `arm64`
- Build mode: release
- Server port: `127.0.0.1:18138`
- Server PID for RSS sampling: `47714`
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
  `target/proof/qwen-0-5b-long-chat-generated-context-probe-1024.log`
- Raw proof exit file:
  `target/proof/qwen-0-5b-long-chat-generated-context-probe-1024.exit`

The proof used a foreground server process held open by the tool session. After
the gate completed and the server was stopped, `lsof -nP -iTCP:18138
-sTCP:LISTEN` returned no listener.

## Server Command

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18138 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 1024 \
  --hard-max-tokens 2048
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
  --token-lengths 1024 \
  --turns 4 \
  --addr 127.0.0.1:18138 \
  --api-key local-secret \
  --rss-pid 47714 \
  --probe-max-tokens 1024 \
  --expect-finish-reason length
```

The gate wrote `0` to
`target/proof/qwen-0-5b-long-chat-generated-context-probe-1024.exit`.

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
| 1 | seed | 1024 | 1 | length | 43 | 1024 | 85341 | 1025 | 2304 | 83321 | 12.301775 | 341393408 | 410206208 | 410189824 |
| 2 | generated | 1024 | 1 | length | 1054 | 1024 | 238822 | 1025 | 86550 | 236803 | 4.328482 | 410189824 | 411795456 | 411779072 |
| 3 | generated | 1024 | 1 | length | 1054 | 1024 | 308148 | 1025 | 118010 | 306127 | 3.348282 | 411779072 | 412844032 | 412844032 |
| 4 | generated | 1024 | 1 | length | 1054 | 1024 | 343298 | 1025 | 78188 | 341266 | 3.003520 | 412844032 | 411500544 | 411500544 |

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

The prompt-token count increased from `43` on the seed turn to `1054` on
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
generated-context proof at 256, 512, and 1024 completion tokens. The 1024-token
run exercises larger carried assistant context than the shorter generated-context
runs and still records token-limit status, usage accounting, timing, RSS, error
recovery, disconnect recovery, fresh reconnect generation, and integrated
`long_chat_summary_run_complete=true`.

This remains partial evidence. The next proof work should apply the same
generated-context matrix to the larger required Tier 1 HTTP model artifacts,
x86_64 generated-context runs, EOS-specific long-chat behavior, and longer
steady-state serving.
