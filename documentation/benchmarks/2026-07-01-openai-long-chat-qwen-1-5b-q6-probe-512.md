# OpenAI Long-Chat Qwen 1.5B Q6 512-Token Probe Gate

## Scope

This run extends the local Qwen2.5 1.5B Q6_K combined reconnect/error
long-chat proof set from 256 tokens to 512 tokens. It uses
`--probe-max-tokens 512`, so the request-error reconnect path, disconnect
reconnect path, and all repeated streaming chat scenarios use the same
512-token budget.

This is one local model and one token length. It does not close the full Tier 1
long-chat gate for larger artifacts, which still requires the 1024-token
combined run for this model, matching combined runs for SmolLM2 1.7B Q4_K_M,
x86_64, and longer steady-state serving.

## Environment

- Date: 2026-07-01
- Commit: `85f7c4e`
- Host: local macOS development machine
- Host architecture: `arm64`
- CPU: `Apple M1 Pro`
- Build mode: release
- Server port: `127.0.0.1:18113`
- Server PID for RSS sampling: `9011`
- Model: `Qwen2.5-1.5B-Instruct-Q6_K`
- Model path: `target/models/qwen2.5-1.5b-instruct-q6_k.gguf`
- API key: `local-secret`
- Raw log: `target/proof/qwen-1-5b-q6-long-chat-probe-512.log`

## Server Command

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18113 \
  --model target/models/qwen2.5-1.5b-instruct-q6_k.gguf \
  --model-id Qwen2.5-1.5B-Instruct-Q6_K \
  --api-key local-secret \
  --default-max-tokens 512 \
  --hard-max-tokens 768
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
  --token-lengths 512 \
  --turns 4 \
  --addr 127.0.0.1:18113 \
  --api-key local-secret \
  --rss-pid 9011 \
  --probe-max-tokens 512 \
  --expect-finish-reason length
```

## Probe Results

Both probes completed and recorded the configured 512-token budget:

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_error_probe_max_tokens=512
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
long_chat_disconnect_probe_max_tokens=512
```

The disconnect probe closed one streaming client after observing generated
content, then completed a fresh reconnect request. Ferrite does not resume
partial SSE generations; this result proves the bounded retry starts a new
streaming request and the server releases the single inference permit for this
512-token local larger-model shape.

## Scenario Results

All four 512-token streaming chat scenarios completed with
`finish_reason=length`, usage accounting for 512 completion tokens, streaming
timing, per-token latency summaries, and RSS samples.

| Turn | Max tokens | Completed | Finish | Total ms | Events | TTFT ms | Stream ms | Tok/s | Lat min ms | Lat p50 ms | Lat p95 ms | Lat max ms | RSS before | RSS after | RSS idle |
| --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 512 | 1 | length | 172625 | 513 | 11589 | 170609 | 3.006862 | 256 | 307 | 344 | 11589 | 1592410112 | 1557610496 | 1557610496 |
| 2 | 512 | 1 | length | 171530 | 513 | 11620 | 169517 | 3.026244 | 254 | 303 | 344 | 11620 | 1557610496 | 1542799360 | 1542799360 |
| 3 | 512 | 1 | length | 178672 | 513 | 13353 | 176658 | 2.903910 | 255 | 315 | 396 | 13353 | 1542799360 | 1533739008 | 1533739008 |
| 4 | 512 | 1 | length | 172674 | 513 | 14393 | 170660 | 3.005976 | 256 | 305 | 341 | 14393 | 1533739008 | 1559117824 | 1559117824 |

Usage was stable for every turn:

- prompt tokens: `43`;
- completion tokens: `512`;
- total tokens: `555`.

The RSS samples stayed in the same approximate range throughout the measured
turns. This is still a single local run, so leak freedom and longer steady-state
memory behavior remain unproven.

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

After stopping the server, `lsof -nP -iTCP:18113 -sTCP:LISTEN` returned no
listener.

## Interpretation

Ferrite now has real local combined long-chat reconnect/error proof for
Qwen2.5-1.5B Q6_K at the 256 and 512 completion-token budgets.

Remaining proof gaps:

- repeat the combined probe-budget shape for this model at 1024 completion
  tokens;
- repeat the combined probe-budget shape for SmolLM2-1.7B Q4_K_M;
- repeat on x86_64;
- run longer steady-state serving and memory-focused samples;
- broaden EOS-specific evidence beyond the local SmolLM2 proof.
