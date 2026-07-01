# OpenAI Long-Chat SmolLM2 1.7B Q4 512-Token Probe Gate

## Scope

This run extends the local SmolLM2 1.7B Q4_K_M combined reconnect/error
long-chat proof set from 256 tokens to 512 tokens. It uses
`--probe-max-tokens 512`, so the request-error reconnect path, disconnect
reconnect path, and all repeated streaming chat scenarios use the same
512-token budget.

The run uses the same full-length prompt shape as the earlier SmolLM2 full
long-chat matrix because the harness default prompt can terminate early with
`finish_reason=stop` for this model. This note records the successful
full-length proof shape, not a stop/EOS result.

This is one local model and one token length. It does not close the full Tier 1
long-chat gate, which still requires the 1024-token combined run for this
model, x86_64, and longer steady-state serving.

## Environment

- Date: 2026-07-01
- Commit: `8dfc9ce`
- Host: local macOS development machine
- Host architecture: `arm64`
- CPU: `Apple M1 Pro`
- Build mode: release
- Server port: `127.0.0.1:18116`
- Server PID for RSS sampling: `37799`
- Model: `SmolLM2-1.7B-Instruct-Q4_K_M`
- Model path: `target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf`
- API key: `local-secret`
- Raw log: `target/proof/smollm-1-7b-q4-long-chat-probe-512.log`

## Server Command

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18116 \
  --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf \
  --model-id SmolLM2-1.7B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 512 \
  --hard-max-tokens 768
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
  --addr 127.0.0.1:18116 \
  --api-key local-secret \
  --rss-pid 37799 \
  --probe-max-tokens 512 \
  --expect-finish-reason length \
  --prompt "Write a concise operational note about CPU inference stability." \
  --assistant-context "CPU inference stability depends on bounded memory use, predictable token latency, and clear server health signals." \
  --follow-up "Continue with reconnect and error-handling risks."
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
512-token local SmolLM2 shape.

## Scenario Results

All four 512-token streaming chat scenarios completed with
`finish_reason=length`, usage accounting for 512 completion tokens, streaming
timing, per-token latency summaries, and RSS samples.

| Turn | Max tokens | Completed | Finish | Total ms | Events | TTFT ms | Stream ms | Tok/s | Lat min ms | Lat p50 ms | Lat p95 ms | Lat max ms | RSS before | RSS after | RSS idle |
| --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 512 | 1 | length | 125269 | 513 | 9286 | 123252 | 4.162182 | 172 | 217 | 275 | 9286 | 1237254144 | 1283342336 | 1283342336 |
| 2 | 512 | 1 | length | 124642 | 513 | 9136 | 122623 | 4.183545 | 168 | 212 | 293 | 9136 | 1283342336 | 1283883008 | 1283883008 |
| 3 | 512 | 1 | length | 128193 | 513 | 10930 | 126177 | 4.065713 | 170 | 215 | 290 | 10930 | 1283883008 | 1290338304 | 1290338304 |
| 4 | 512 | 1 | length | 129303 | 513 | 9644 | 127289 | 4.030187 | 171 | 222 | 284 | 9644 | 1290338304 | 1308639232 | 1308639232 |

Usage was stable for every turn:

- prompt tokens: `53`;
- completion tokens: `512`;
- total tokens: `565`.

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

After stopping the server, `lsof -nP -iTCP:18116 -sTCP:LISTEN` returned no
listener.

## Interpretation

Ferrite now has real local combined long-chat reconnect/error proof for
SmolLM2-1.7B Q4_K_M at the 256 and 512 completion-token budgets.

Remaining proof gaps:

- repeat the combined probe-budget shape for this model at 1024 completion
  tokens;
- repeat on x86_64;
- run longer steady-state serving and memory-focused samples;
- broaden EOS-specific evidence beyond the local SmolLM2 proof.
