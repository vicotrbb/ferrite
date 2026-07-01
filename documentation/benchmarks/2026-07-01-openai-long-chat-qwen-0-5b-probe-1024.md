# OpenAI Long-Chat Qwen 0.5B 1024-Token Probe Gate

## Scope

This run completes the local Qwen2.5 0.5B Q4_K_M combined reconnect/error
long-chat proof set by adding the 1024-token budget. It uses
`--probe-max-tokens 1024`, so the request-error reconnect path, disconnect
reconnect path, and all repeated streaming chat scenarios use the same
1024-token budget.

This completes the 256/512/1024 combined reconnect/error shape for this one
local model. It does not close the full Tier 1 long-chat gate, which still
requires the same combined proof shape for the larger Tier 1 artifacts, x86_64,
and longer steady-state serving.

## Environment

- Date: 2026-07-01
- Commit: `151def4`
- Host: local macOS development machine
- Host architecture: `arm64`
- CPU: `Apple M1 Pro`
- Build mode: release
- Server port: `127.0.0.1:18108`
- Server PID for RSS sampling: `54773`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Model path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- API key: `local-secret`
- Raw log: `target/proof/qwen-0-5b-long-chat-probe-1024.log`

## Server Command

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18108 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 1024 \
  --hard-max-tokens 1280
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
  --addr 127.0.0.1:18108 \
  --api-key local-secret \
  --rss-pid 54773 \
  --probe-max-tokens 1024 \
  --expect-finish-reason length
```

## Probe Results

Both probes completed and recorded the configured 1024-token budget:

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_error_probe_max_tokens=1024
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
long_chat_disconnect_probe_max_tokens=1024
```

The disconnect probe closed one streaming client after observing generated
content, then completed a fresh reconnect request. Ferrite does not resume
partial SSE generations; this result proves the bounded retry starts a new
streaming request and the server releases the single inference permit for this
1024-token local shape.

## Scenario Results

All four 1024-token streaming chat scenarios completed with
`finish_reason=length`, usage accounting for 1024 completion tokens, streaming
timing, and RSS samples.

| Turn | Max tokens | Completed | Finish | Total ms | Events | TTFT ms | Stream ms | Tok/s | RSS before | RSS after | RSS idle |
| --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 1024 | 1 | length | 89266 | 1025 | 2048 | 87251 | 11.747676 | 410140672 | 411697152 | 411697152 |
| 2 | 1024 | 1 | length | 84639 | 1025 | 1945 | 82623 | 12.405713 | 411697152 | 423837696 | 423247872 |
| 3 | 1024 | 1 | length | 93928 | 1025 | 2507 | 91912 | 11.151884 | 423247872 | 410222592 | 410222592 |
| 4 | 1024 | 1 | length | 93860 | 1025 | 1942 | 91845 | 11.160071 | 410222592 | 410157056 | 410157056 |

Usage was stable for every turn:

- prompt tokens: `43`;
- completion tokens: `1024`;
- total tokens: `1067`.

Unlike the 256-token and 512-token combined runs, this run did not show an
obvious low RSS sampling anomaly. It is still a single local run, so leak
freedom and longer steady-state memory behavior remain unproven.

## Integrated Summary

```text
long_chat_summary_planned_scenarios=4
long_chat_summary_completed_scenarios=4
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

After stopping the server, `lsof -nP -iTCP:18108 -sTCP:LISTEN` returned no
listener.

## Interpretation

Ferrite now has real local combined long-chat reconnect/error proof for
Qwen2.5-0.5B Q4_K_M at 256, 512, and 1024 completion-token budgets.

Remaining proof gaps:

- repeat the combined probe-budget shape for Qwen2.5-1.5B Q8_0,
  Qwen2.5-1.5B Q6_K, and SmolLM2-1.7B Q4_K_M;
- repeat on x86_64;
- run longer steady-state serving and memory-focused samples;
- broaden EOS-specific evidence beyond the local SmolLM2 proof.
