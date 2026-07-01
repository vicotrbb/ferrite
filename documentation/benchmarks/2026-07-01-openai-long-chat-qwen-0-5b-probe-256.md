# OpenAI Long-Chat Qwen 0.5B 256-Token Probe Gate

## Scope

This run exercises the long-chat gate's combined reconnect and error behavior
at a 256-token streaming chat length for Qwen2.5 0.5B Q4_K_M. It uses the
`--probe-max-tokens 256` harness option so the error reconnect request and the
disconnect reconnect request use the same token budget as the repeated
long-chat scenarios.

This is one local model and one token length. It does not close the full Tier 1
long-chat gate, which still requires the same combined proof shape across 512
and 1024 tokens, the larger Tier 1 artifacts, x86_64, and longer steady-state
serving.

## Environment

- Date: 2026-07-01
- Commit: `ce02347`
- Host: local macOS development machine
- Host architecture: `arm64`
- CPU: `Apple M1 Pro`
- Build mode: release
- Server port: `127.0.0.1:18106`
- Server PID for RSS sampling: `50066`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Model path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- API key: `local-secret`
- Raw log: `target/proof/qwen-0-5b-long-chat-probe-256.log`

## Server Command

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18106 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 256 \
  --hard-max-tokens 512
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
  --token-lengths 256 \
  --turns 4 \
  --addr 127.0.0.1:18106 \
  --api-key local-secret \
  --rss-pid 50066 \
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
streaming request and the server releases the single inference permit.

## Scenario Results

All four 256-token streaming chat scenarios completed with
`finish_reason=length`, usage accounting for 256 completion tokens, streaming
timing, and RSS samples.

| Turn | Max tokens | Completed | Finish | Total ms | Events | TTFT ms | Stream ms | Tok/s | RSS before | RSS after | RSS idle |
| --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 256 | 1 | length | 22637 | 257 | 2706 | 20625 | 12.460538 | 415318016 | 422117376 | 422051840 |
| 2 | 256 | 1 | length | 23840 | 257 | 2271 | 21826 | 11.774623 | 422051840 | 407126016 | 406388736 |
| 3 | 256 | 1 | length | 25333 | 257 | 3238 | 23321 | 11.020104 | 406388736 | 406421504 | 406241280 |
| 4 | 256 | 1 | length | 24355 | 257 | 2770 | 22336 | 11.505924 | 406241280 | 406241280 | 5750784 |

Usage was stable for every turn:

- prompt tokens: `43`;
- completion tokens: `256`;
- total tokens: `299`.

The fourth idle RSS sample is anomalously low compared with the surrounding
server samples. The raw log is preserved, and the run is still useful as
latency, finish-reason, usage, probe-token-budget, and reconnect/error evidence.
Future memory-focused runs should repeat the same shape and inspect the RSS
sampler if this low idle value recurs.

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

After stopping the server, `lsof -nP -iTCP:18106 -sTCP:LISTEN` returned no
listener.

## Interpretation

Ferrite now has one real local combined long-chat proof where the reconnect and
error probes use the same 256-token budget as the repeated streaming chat
scenarios. This tightens the previous integrated stop evidence, which used
short probe requests.

Remaining proof gaps:

- repeat the combined probe-budget shape for 512 and 1024 tokens;
- repeat the combined probe-budget shape for Qwen2.5-1.5B Q8_0,
  Qwen2.5-1.5B Q6_K, and SmolLM2-1.7B Q4_K_M;
- repeat on x86_64;
- run longer steady-state serving and memory-focused samples;
- broaden EOS-specific evidence beyond the local SmolLM2 proof.
