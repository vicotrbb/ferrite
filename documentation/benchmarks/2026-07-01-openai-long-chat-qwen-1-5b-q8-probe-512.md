# OpenAI Long-Chat Qwen 1.5B Q8 512-Token Probe Gate

## Scope

This run extends the combined reconnect/error long-chat proof set for
Qwen2.5 1.5B Q8_0 from 256 tokens to 512 tokens. It uses
`--probe-max-tokens 512`, so the request-error reconnect path, disconnect
reconnect path, and all repeated streaming chat scenarios use the same
512-token budget.

This is one local model and one token length. It does not close the full Tier 1
long-chat gate for larger artifacts, which still requires a 1024-token combined
run for this model, matching combined runs for Qwen2.5 1.5B Q6_K and SmolLM2
1.7B Q4_K_M, x86_64, and longer steady-state serving.

## Environment

- Date: 2026-07-01
- Commit: `25133e7`
- Host: local macOS development machine
- Host architecture: `arm64`
- CPU: `Apple M1 Pro`
- Build mode: release
- Server port: `127.0.0.1:18110`
- Server PID for RSS sampling: `76775`
- Model: `Qwen2.5-1.5B-Instruct-Q8_0`
- Model path: `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
- API key: `local-secret`
- Raw log: `target/proof/qwen-1-5b-q8-long-chat-probe-512.log`

## Server Command

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18110 \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --model-id Qwen2.5-1.5B-Instruct-Q8_0 \
  --api-key local-secret \
  --default-max-tokens 512 \
  --hard-max-tokens 768
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
  --token-lengths 512 \
  --turns 4 \
  --addr 127.0.0.1:18110 \
  --api-key local-secret \
  --rss-pid 76775 \
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
timing, and RSS samples.

| Turn | Max tokens | Completed | Finish | Total ms | Events | TTFT ms | Stream ms | Tok/s | RSS before | RSS after | RSS idle |
| --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 512 | 1 | length | 76670 | 513 | 4010 | 74637 | 6.873195 | 1972387840 | 1689124864 | 1689124864 |
| 2 | 512 | 1 | length | 71895 | 513 | 4503 | 69880 | 7.341053 | 1689124864 | 1717075968 | 1717075968 |
| 3 | 512 | 1 | length | 156592 | 513 | 3964 | 154578 | 3.318705 | 1717075968 | 1683030016 | 1683030016 |
| 4 | 512 | 1 | length | 87378 | 513 | 4748 | 85342 | 6.011061 | 1683030016 | 1708048384 | 1707819008 |

Usage was stable for every turn:

- prompt tokens: `43`;
- completion tokens: `512`;
- total tokens: `555`.

The run had one notably slower turn, with turn 3 streaming at 3.318705 tok/s
versus 6.0-7.3 tok/s for the other turns. This is recorded as observed local
latency evidence only; it is not a throughput readiness claim.

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

After stopping the server, `lsof -nP -iTCP:18110 -sTCP:LISTEN` returned no
listener.

## Interpretation

Ferrite now has real local combined long-chat reconnect/error proof for
Qwen2.5-1.5B Q8_0 at 256 and 512 completion-token budgets.

Remaining proof gaps:

- repeat the combined probe-budget shape for this model at 1024 completion
  tokens;
- repeat the combined probe-budget shape for Qwen2.5-1.5B Q6_K and
  SmolLM2-1.7B Q4_K_M;
- repeat on x86_64;
- run longer steady-state serving and memory-focused samples;
- broaden EOS-specific evidence beyond the local SmolLM2 proof.
