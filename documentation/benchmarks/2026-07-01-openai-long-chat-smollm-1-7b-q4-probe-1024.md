# OpenAI Long-Chat SmolLM2 1.7B Q4 1024-Token Probe Gate

## Scope

This run completes the local SmolLM2 1.7B Q4_K_M combined reconnect/error
long-chat proof set by adding the 1024-token budget. It uses
`--probe-max-tokens 1024`, so the request-error reconnect path, disconnect
reconnect path, and all repeated streaming chat scenarios use the same
1024-token budget.

The run uses the same full-length prompt shape as the earlier SmolLM2 full
long-chat matrix because the harness default prompt can terminate early with
`finish_reason=stop` for this model. This note records the successful
full-length proof shape, not a stop/EOS result.

This completes the 256/512/1024 combined reconnect/error shape for this one
local Tier 1 artifact. It does not close the full Tier 1 long-chat gate, which
still requires x86_64 and longer steady-state serving evidence.

## Environment

- Date: 2026-07-01
- Commit: `0523665`
- Host: local macOS development machine
- Host architecture: `arm64`
- CPU: `Apple M1 Pro`
- Build mode: release
- Server port: `127.0.0.1:18117`
- Server PID for RSS sampling: `42126`
- Model: `SmolLM2-1.7B-Instruct-Q4_K_M`
- Model path: `target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf`
- API key: `local-secret`
- Raw log: `target/proof/smollm-1-7b-q4-long-chat-probe-1024.log`

## Server Command

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18117 \
  --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf \
  --model-id SmolLM2-1.7B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 1024 \
  --hard-max-tokens 1280
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
  --token-lengths 1024 \
  --turns 4 \
  --addr 127.0.0.1:18117 \
  --api-key local-secret \
  --rss-pid 42126 \
  --probe-max-tokens 1024 \
  --expect-finish-reason length \
  --prompt "Write a concise operational note about CPU inference stability." \
  --assistant-context "CPU inference stability depends on bounded memory use, predictable token latency, and clear server health signals." \
  --follow-up "Continue with reconnect and error-handling risks."
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
1024-token local SmolLM2 shape.

## Scenario Results

All four 1024-token streaming chat scenarios completed with
`finish_reason=length`, usage accounting for 1024 completion tokens, streaming
timing, per-token latency summaries, and RSS samples.

| Turn | Max tokens | Completed | Finish | Total ms | Events | TTFT ms | Stream ms | Tok/s | Lat min ms | Lat p50 ms | Lat p95 ms | Lat max ms | RSS before | RSS after | RSS idle |
| --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 1024 | 1 | length | 332324 | 1025 | 9438 | 330306 | 3.103177 | 172 | 295 | 421 | 9438 | 1299005440 | 1349009408 | 1349009408 |
| 2 | 1024 | 1 | length | 312585 | 1025 | 9578 | 310566 | 3.300416 | 174 | 283 | 423 | 9578 | 1349009408 | 1408237568 | 1408237568 |
| 3 | 1024 | 1 | length | 311030 | 1025 | 9387 | 309013 | 3.317008 | 165 | 289 | 407 | 9387 | 1408237568 | 1465106432 | 1465106432 |
| 4 | 1024 | 1 | length | 303349 | 1025 | 9211 | 301334 | 3.401539 | 168 | 274 | 423 | 9211 | 1465106432 | 1474592768 | 1474592768 |

Usage was stable for every turn:

- prompt tokens: `53`;
- completion tokens: `1024`;
- total tokens: `1077`.

RSS increased across the measured 1024-token turns, from `1299005440` bytes
before turn 1 to `1474592768` bytes after the final idle sample. This is single
local-run behavior, not leak-freedom proof; longer steady-state and
memory-focused reruns remain required before making memory-posture claims.

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

After stopping the server, `lsof -nP -iTCP:18117 -sTCP:LISTEN` returned no
listener.

## Interpretation

Ferrite now has real local combined long-chat reconnect/error proof for
SmolLM2-1.7B Q4_K_M at 256, 512, and 1024 completion-token budgets.

Remaining proof gaps:

- repeat on x86_64;
- run longer steady-state serving and memory-focused samples;
- broaden EOS-specific evidence beyond the local SmolLM2 proof.
