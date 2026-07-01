# OpenAI Long-Chat Qwen 1.5B Q6 1024-Token Probe Gate

## Scope

This run completes the local Qwen2.5 1.5B Q6_K combined reconnect/error
long-chat proof set by adding the 1024-token budget. It uses
`--probe-max-tokens 1024`, so the request-error reconnect path, disconnect
reconnect path, and all repeated streaming chat scenarios use the same
1024-token budget.

This completes the 256/512/1024 combined reconnect/error shape for this one
local larger Tier 1 artifact. It does not close the full Tier 1 long-chat gate,
which still requires matching combined runs for SmolLM2 1.7B Q4_K_M, x86_64,
and longer steady-state serving.

## Environment

- Date: 2026-07-01
- Commit: `d167859`
- Host: local macOS development machine
- Host architecture: `arm64`
- CPU: `Apple M1 Pro`
- Build mode: release
- Server port: `127.0.0.1:18114`
- Server PID for RSS sampling: `18882`
- Model: `Qwen2.5-1.5B-Instruct-Q6_K`
- Model path: `target/models/qwen2.5-1.5b-instruct-q6_k.gguf`
- API key: `local-secret`
- Raw log: `target/proof/qwen-1-5b-q6-long-chat-probe-1024.log`

## Server Command

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18114 \
  --model target/models/qwen2.5-1.5b-instruct-q6_k.gguf \
  --model-id Qwen2.5-1.5B-Instruct-Q6_K \
  --api-key local-secret \
  --default-max-tokens 1024 \
  --hard-max-tokens 1280
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
  --addr 127.0.0.1:18114 \
  --api-key local-secret \
  --rss-pid 18882 \
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
timing, per-token latency summaries, and RSS samples.

| Turn | Max tokens | Completed | Finish | Total ms | Events | TTFT ms | Stream ms | Tok/s | Lat min ms | Lat p50 ms | Lat p95 ms | Lat max ms | RSS before | RSS after | RSS idle |
| --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 1024 | 1 | length | 378173 | 1025 | 11591 | 376155 | 2.724937 | 255 | 336 | 421 | 11591 | 1595277312 | 1560412160 | 1560412160 |
| 2 | 1024 | 1 | length | 362228 | 1025 | 16433 | 360210 | 2.845556 | 258 | 328 | 397 | 16433 | 1560412160 | 1587396608 | 1587396608 |
| 3 | 1024 | 1 | length | 362161 | 1025 | 11538 | 360144 | 2.846083 | 255 | 339 | 415 | 11538 | 1587396608 | 1577910272 | 1577910272 |
| 4 | 1024 | 1 | length | 357335 | 1025 | 11762 | 355317 | 2.884740 | 257 | 333 | 391 | 11762 | 1577910272 | 1597587456 | 1597587456 |

Usage was stable for every turn:

- prompt tokens: `43`;
- completion tokens: `1024`;
- total tokens: `1067`.

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

After stopping the server, `lsof -nP -iTCP:18114 -sTCP:LISTEN` returned no
listener.

## Interpretation

Ferrite now has real local combined long-chat reconnect/error proof for
Qwen2.5-1.5B Q6_K at 256, 512, and 1024 completion-token budgets.

Remaining proof gaps:

- repeat the combined probe-budget shape for SmolLM2-1.7B Q4_K_M;
- repeat on x86_64;
- run longer steady-state serving and memory-focused samples;
- broaden EOS-specific evidence beyond the local SmolLM2 proof.
