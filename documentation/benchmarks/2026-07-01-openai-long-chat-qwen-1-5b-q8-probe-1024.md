# OpenAI Long-Chat Qwen 1.5B Q8 1024-Token Probe Gate

## Scope

This run completes the local Qwen2.5 1.5B Q8_0 combined reconnect/error
long-chat proof set by adding the 1024-token budget. It uses
`--probe-max-tokens 1024`, so the request-error reconnect path, disconnect
reconnect path, and all repeated streaming chat scenarios use the same
1024-token budget.

This completes the 256/512/1024 combined reconnect/error shape for this one
local larger Tier 1 artifact. It does not close the full Tier 1 long-chat gate,
which still requires matching combined runs for Qwen2.5 1.5B Q6_K and SmolLM2
1.7B Q4_K_M, x86_64, and longer steady-state serving.

## Environment

- Date: 2026-07-01
- Commit: `5308200`
- Host: local macOS development machine
- Host architecture: `arm64`
- CPU: `Apple M1 Pro`
- Build mode: release
- Server port: `127.0.0.1:18111`
- Server PID for RSS sampling: `86513`
- Model: `Qwen2.5-1.5B-Instruct-Q8_0`
- Model path: `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
- API key: `local-secret`
- Raw log: `target/proof/qwen-1-5b-q8-long-chat-probe-1024.log`

## Server Command

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18111 \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --model-id Qwen2.5-1.5B-Instruct-Q8_0 \
  --api-key local-secret \
  --default-max-tokens 1024 \
  --hard-max-tokens 1280
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
  --token-lengths 1024 \
  --turns 4 \
  --addr 127.0.0.1:18111 \
  --api-key local-secret \
  --rss-pid 86513 \
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
1024-token local larger-model shape.

## Scenario Results

All four 1024-token streaming chat scenarios completed with
`finish_reason=length`, usage accounting for 1024 completion tokens, streaming
timing, and RSS samples.

| Turn | Max tokens | Completed | Finish | Total ms | Events | TTFT ms | Stream ms | Tok/s | RSS before | RSS after | RSS idle |
| --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 1024 | 1 | length | 184473 | 1025 | 4300 | 182455 | 5.617821 | 1707311104 | 1729085440 | 1729085440 |
| 2 | 1024 | 1 | length | 168383 | 1025 | 3921 | 166367 | 6.161068 | 1729085440 | 1718108160 | 1718108160 |
| 3 | 1024 | 1 | length | 166871 | 1025 | 3888 | 164856 | 6.217544 | 1718108160 | 1743945728 | 1743945728 |
| 4 | 1024 | 1 | length | 165080 | 1025 | 3834 | 163063 | 6.285907 | 1743945728 | 1759969280 | 1759969280 |

Usage was stable for every turn:

- prompt tokens: `43`;
- completion tokens: `1024`;
- total tokens: `1067`.

The 1024-token run did not show the low RSS sampling anomaly observed in the
earlier 256-token Q8_0 run. It remains a single local run, so leak freedom and
longer steady-state memory behavior remain unproven.

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

After stopping the server, `lsof -nP -iTCP:18111 -sTCP:LISTEN` returned no
listener.

## Interpretation

Ferrite now has real local combined long-chat reconnect/error proof for
Qwen2.5-1.5B Q8_0 at 256, 512, and 1024 completion-token budgets.

Remaining proof gaps:

- repeat the combined probe-budget shape for Qwen2.5-1.5B Q6_K and
  SmolLM2-1.7B Q4_K_M;
- repeat on x86_64;
- run longer steady-state serving and memory-focused samples;
- broaden EOS-specific evidence beyond the local SmolLM2 proof.
