# OpenAI Long-Chat Qwen 0.5B 512-Token Probe Gate

## Scope

This run extends the Qwen2.5 0.5B Q4_K_M combined reconnect/error long-chat
proof from 256 tokens to 512 tokens. It uses `--probe-max-tokens 512`, so the
request-error reconnect path, disconnect reconnect path, and all repeated
streaming chat scenarios use the same 512-token budget.

This is one local model and one token length. It does not close the full Tier 1
long-chat gate, which still requires the same combined proof shape for 1024
tokens, the larger Tier 1 artifacts, x86_64, and longer steady-state serving.

## Environment

- Date: 2026-07-01
- Commit: `5f67e98`
- Host: local macOS development machine
- Host architecture: `arm64`
- CPU: `Apple M1 Pro`
- Build mode: release
- Server port: `127.0.0.1:18107`
- Server PID for RSS sampling: `45881`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Model path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- API key: `local-secret`
- Raw log: `target/proof/qwen-0-5b-long-chat-probe-512.log`

## Server Command

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18107 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 512 \
  --hard-max-tokens 768
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
  --token-lengths 512 \
  --turns 4 \
  --addr 127.0.0.1:18107 \
  --api-key local-secret \
  --rss-pid 45881 \
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
512-token local shape.

## Scenario Results

All four 512-token streaming chat scenarios completed with
`finish_reason=length`, usage accounting for 512 completion tokens, streaming
timing, and RSS samples.

| Turn | Max tokens | Completed | Finish | Total ms | Events | TTFT ms | Stream ms | Tok/s | RSS before | RSS after | RSS idle |
| --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 512 | 1 | length | 40388 | 513 | 2478 | 38373 | 13.368512 | 426901504 | 429375488 | 429375488 |
| 2 | 512 | 1 | length | 55720 | 513 | 1996 | 53693 | 9.554162 | 429375488 | 407076864 | 114475008 |
| 3 | 512 | 1 | length | 113491 | 513 | 19886 | 111468 | 4.602215 | 114475008 | 409944064 | 409944064 |
| 4 | 512 | 1 | length | 48649 | 513 | 2268 | 46631 | 11.001231 | 409944064 | 409829376 | 409632768 |

Usage was stable for every turn:

- prompt tokens: `43`;
- completion tokens: `512`;
- total tokens: `555`.

The second idle RSS sample and third before-request RSS sample are anomalously
low compared with the surrounding server samples, matching the kind of RSS
sampling anomaly seen in the 256-token run. The run remains useful as
finish-reason, usage, latency, probe-token-budget, and reconnect/error evidence.
Future memory-focused runs should repeat this shape and inspect the RSS sampler
before treating those low samples as memory posture proof.

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

After stopping the server, `lsof -nP -iTCP:18107 -sTCP:LISTEN` returned no
listener.

## Interpretation

Ferrite now has real local combined long-chat reconnect/error proof for
Qwen2.5-0.5B Q4_K_M at both 256 and 512 completion-token budgets.

Remaining proof gaps:

- repeat the combined probe-budget shape for 1024 tokens;
- repeat the combined probe-budget shape for Qwen2.5-1.5B Q8_0,
  Qwen2.5-1.5B Q6_K, and SmolLM2-1.7B Q4_K_M;
- repeat on x86_64;
- run longer steady-state serving and memory-focused samples;
- broaden EOS-specific evidence beyond the local SmolLM2 proof.
