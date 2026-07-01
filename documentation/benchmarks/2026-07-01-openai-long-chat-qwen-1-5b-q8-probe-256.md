# OpenAI Long-Chat Qwen 1.5B Q8 256-Token Probe Gate

## Scope

This run starts the combined reconnect/error long-chat proof set for the larger
Qwen2.5 1.5B Q8_0 Tier 1 artifact. It uses `--probe-max-tokens 256`, so the
request-error reconnect path, disconnect reconnect path, and all repeated
streaming chat scenarios use the same 256-token budget.

This is one local model and one token length. It does not close the full Tier 1
long-chat gate for larger artifacts, which still requires 512 and 1024-token
combined runs for this model, matching combined runs for Qwen2.5 1.5B Q6_K and
SmolLM2 1.7B Q4_K_M, x86_64, and longer steady-state serving.

## Environment

- Date: 2026-07-01
- Commit: `d56254d`
- Host: local macOS development machine
- Host architecture: `arm64`
- CPU: `Apple M1 Pro`
- Build mode: release
- Server port: `127.0.0.1:18109`
- Server PID for RSS sampling: `65944`
- Model: `Qwen2.5-1.5B-Instruct-Q8_0`
- Model path: `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
- API key: `local-secret`
- Raw log: `target/proof/qwen-1-5b-q8-long-chat-probe-256.log`

## Server Command

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18109 \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --model-id Qwen2.5-1.5B-Instruct-Q8_0 \
  --api-key local-secret \
  --default-max-tokens 256 \
  --hard-max-tokens 512
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
  --token-lengths 256 \
  --turns 4 \
  --addr 127.0.0.1:18109 \
  --api-key local-secret \
  --rss-pid 65944 \
  --probe-max-tokens 256 \
  --expect-finish-reason length
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
256-token local larger-model shape.

## Scenario Results

All four 256-token streaming chat scenarios completed with
`finish_reason=length`, usage accounting for 256 completion tokens, streaming
timing, and RSS samples.

| Turn | Max tokens | Completed | Finish | Total ms | Events | TTFT ms | Stream ms | Tok/s | RSS before | RSS after | RSS idle |
| --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 256 | 1 | length | 55733 | 257 | 5682 | 53675 | 4.788052 | 1682063360 | 1667268608 | 369065984 |
| 2 | 256 | 1 | length | 138884 | 257 | 7277 | 136774 | 1.879004 | 369065984 | 1115340800 | 12042240 |
| 3 | 256 | 1 | length | 100928 | 257 | 28057 | 98893 | 2.598751 | 9830400 | 1667547136 | 1667547136 |
| 4 | 256 | 1 | length | 43308 | 257 | 5028 | 41295 | 6.223446 | 1667547136 | 1669726208 | 1669726208 |

Usage was stable for every turn:

- prompt tokens: `43`;
- completion tokens: `256`;
- total tokens: `299`.

The RSS samples include obvious anomalously low values in the turn 1 idle, turn
2 before/idle, and turn 3 before samples. This run is therefore evidence for
finish reason, usage accounting, streaming latency, probe-token budget, and
reconnect/error behavior, not memory posture. A memory-focused rerun should
inspect the RSS sampler before treating low samples as meaningful memory
movement.

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

After stopping the server, `lsof -nP -iTCP:18109 -sTCP:LISTEN` returned no
listener.

## Interpretation

Ferrite now has real local combined long-chat reconnect/error proof for
Qwen2.5-1.5B Q8_0 at the 256 completion-token budget.

Remaining proof gaps:

- repeat the combined probe-budget shape for this model at 512 and 1024
  completion tokens;
- repeat the combined probe-budget shape for Qwen2.5-1.5B Q6_K and
  SmolLM2-1.7B Q4_K_M;
- repeat on x86_64;
- run longer steady-state serving and memory-focused samples;
- broaden EOS-specific evidence beyond the local SmolLM2 proof.
