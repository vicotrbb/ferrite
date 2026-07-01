# OpenAI Long-Chat SmolLM2 1.7B Q4 256-Token Probe Gate

## Scope

This run starts the combined reconnect/error long-chat proof set for the
SmolLM2 1.7B Q4_K_M Tier 1 artifact. It uses `--probe-max-tokens 256`, so the
request-error reconnect path, disconnect reconnect path, and all repeated
streaming chat scenarios use the same 256-token budget.

The run uses the same full-length prompt shape as the earlier SmolLM2 full
long-chat matrix because the harness default prompt can terminate early with
`finish_reason=stop` for this model. This note records the successful
full-length proof shape, not a stop/EOS result.

This is one local model and one token length. It does not close the full Tier 1
long-chat gate, which still requires the 512 and 1024-token combined runs for
this model, x86_64, and longer steady-state serving.

## Environment

- Date: 2026-07-01
- Commit: `0973318`
- Host: local macOS development machine
- Host architecture: `arm64`
- CPU: `Apple M1 Pro`
- Build mode: release
- Server port: `127.0.0.1:18115`
- Server PID for RSS sampling: `34068`
- Model: `SmolLM2-1.7B-Instruct-Q4_K_M`
- Model path: `target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf`
- API key: `local-secret`
- Raw log: `target/proof/smollm-1-7b-q4-long-chat-probe-256.log`

## Server Command

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18115 \
  --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf \
  --model-id SmolLM2-1.7B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 256 \
  --hard-max-tokens 512
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
  --token-lengths 256 \
  --turns 4 \
  --addr 127.0.0.1:18115 \
  --api-key local-secret \
  --rss-pid 34068 \
  --probe-max-tokens 256 \
  --expect-finish-reason length \
  --prompt "Write a concise operational note about CPU inference stability." \
  --assistant-context "CPU inference stability depends on bounded memory use, predictable token latency, and clear server health signals." \
  --follow-up "Continue with reconnect and error-handling risks."
```

## Probe Results

Both probes completed and recorded the configured 256-token budget:

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_error_probe_max_tokens=256
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
long_chat_disconnect_probe_max_tokens=256
```

The disconnect probe closed one streaming client after observing generated
content, then completed a fresh reconnect request. Ferrite does not resume
partial SSE generations; this result proves the bounded retry starts a new
streaming request and the server releases the single inference permit for this
256-token local SmolLM2 shape.

## Scenario Results

All four 256-token streaming chat scenarios completed with
`finish_reason=length`, usage accounting for 256 completion tokens, streaming
timing, per-token latency summaries, and RSS samples.

| Turn | Max tokens | Completed | Finish | Total ms | Events | TTFT ms | Stream ms | Tok/s | Lat min ms | Lat p50 ms | Lat p95 ms | Lat max ms | RSS before | RSS after | RSS idle |
| --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 256 | 1 | length | 62675 | 257 | 9731 | 60662 | 4.236571 | 172 | 198 | 223 | 9731 | 1173995520 | 1190789120 | 1190789120 |
| 2 | 256 | 1 | length | 63848 | 257 | 9544 | 61835 | 4.156211 | 171 | 201 | 232 | 9544 | 1190789120 | 1181696000 | 1181696000 |
| 3 | 256 | 1 | length | 62472 | 257 | 9283 | 60461 | 4.250635 | 172 | 200 | 220 | 9283 | 1181696000 | 1195851776 | 1195851776 |
| 4 | 256 | 1 | length | 62850 | 257 | 9557 | 60838 | 4.224285 | 168 | 197 | 221 | 9557 | 1195851776 | 1206091776 | 1206091776 |

Usage was stable for every turn:

- prompt tokens: `53`;
- completion tokens: `256`;
- total tokens: `309`.

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

After stopping the server, `lsof -nP -iTCP:18115 -sTCP:LISTEN` returned no
listener.

## Interpretation

Ferrite now has real local combined long-chat reconnect/error proof for
SmolLM2-1.7B Q4_K_M at the 256 completion-token budget.

Remaining proof gaps:

- repeat the combined probe-budget shape for this model at 512 and 1024
  completion tokens;
- repeat on x86_64;
- run longer steady-state serving and memory-focused samples;
- broaden EOS-specific evidence beyond the local SmolLM2 proof.
