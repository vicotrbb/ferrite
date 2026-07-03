# Benchmark: Local Qwen 0.5B Stop Probe 64

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Exercise explicit stop behavior through Ferrite's OpenAI-compatible streaming
chat endpoint. This bounded local Qwen2.5-0.5B slice requires
`finish_reason=stop` across four turns and proves the run stops before the
configured token cap.

## Environment

- Ferrite runtime code commit: `8c1cc4f`
- Host: local macOS workspace
- Server: `127.0.0.1:18235`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Served model id: `qwen2.5-0.5b-q4_k_m`
- Proof directory: `target/proof/local-qwen05-stop-probe-64-2026-07-03/`
- Server binary SHA256:
  `dec0167a646244de6392efbfe5b1549c4064dbab729de894aaa87c02c988b473`
- Gate binary SHA256:
  `7a953e710de9210b2832d61fa55dc89a8f835d5207a7e18659d9f9480ab03e97`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

The local server was stopped after the run. A final bind-specific listener
check returned no listener on `127.0.0.1:18235`.

## Server

```sh
RUST_LOG=info target/release/ferrite-server \
  --bind 127.0.0.1:18235 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id qwen2.5-0.5b-q4_k_m \
  --api-key local-secret \
  --default-max-tokens 128 \
  --hard-max-tokens 256 \
  --inference-wait-ms 120000 \
  --experimental-prefix-cache
```

Readiness:

```text
GET /health -> {"status":"ok","ready":true,"model":"qwen2.5-0.5b-q4_k_m"}
```

## Gate Command

```sh
target/release/ferrite-openai-long-chat-gate \
  --addr 127.0.0.1:18235 \
  --api-key local-secret \
  --execute \
  --models qwen2.5-0.5b-q4_k_m \
  --require-models qwen2.5-0.5b-q4_k_m \
  --prompt 'Reply with only FERRITE_STOP.' \
  --assistant-context 'FERRITE_STOP' \
  --follow-up 'Reply with only FERRITE_STOP.' \
  --stop FERRITE_STOP \
  --expect-finish-reason stop \
  --token-lengths 64 \
  --require-token-lengths 64 \
  --turns 4 \
  --rss-pid <server-pid> \
  --prompt-cache-trace \
  --proof-log target/proof/local-qwen05-stop-probe-64-2026-07-03/long-chat.log \
  --proof-exit-code target/proof/local-qwen05-stop-probe-64-2026-07-03/long-chat.exit
```

## Artifacts

| Artifact | Lines | SHA256 |
| --- | ---: | --- |
| `target/proof/local-qwen05-stop-probe-64-2026-07-03/long-chat.log` | 214 | `9b4e46072579bf082c76d21bc65243dc371ce86b23ded8e5e6f7da923f439978` |
| `target/proof/local-qwen05-stop-probe-64-2026-07-03/gate.stdout` | 214 | `9b4e46072579bf082c76d21bc65243dc371ce86b23ded8e5e6f7da923f439978` |
| `target/proof/local-qwen05-stop-probe-64-2026-07-03/gate.stderr` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `target/proof/local-qwen05-stop-probe-64-2026-07-03/server.log` | 4 | `613cd805da1b6e893598c80e84a2df0eb665539eb8c7714e3e30035d243a6f0c` |
| `target/proof/local-qwen05-stop-probe-64-2026-07-03/server.stdout` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `target/proof/local-qwen05-stop-probe-64-2026-07-03/health.json` | 0 | `e3284eada962df1c75177574e65d3c528a2dcc0fb990143e5877c096413857b4` |
| `target/proof/local-qwen05-stop-probe-64-2026-07-03/long-chat.exit` | 1 | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |
| `target/proof/local-qwen05-stop-probe-64-2026-07-03/gate-command.exit` | 1 | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |

Both exit-code files contained `0`. The health file contains one JSON payload
without a trailing newline, so `wc -l` reports `0`.

## Scenario Results

| Turn | Finish | Completion tokens | Response bytes | TTFT ms | Tok/s | RSS before | RSS after | RSS idle |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | stop | 20 | 33 | 1096 | 9.958581 | 478625792 | 485670912 | 485670912 |
| 2 | stop | 20 | 33 | 1136 | 9.670331 | 485670912 | 488144896 | 488144896 |
| 3 | stop | 20 | 33 | 2 | 24.717162 | 488144896 | 490422272 | 490422272 |
| 4 | stop | 20 | 33 | 2 | 24.583440 | 490422272 | 490848256 | 490848256 |

Every turn reported the same generated-response hash:

```text
long_chat_result_generated_response_hash=fnv64:51fcb343638dd399
```

## Summary

```text
long_chat_summary_planned_scenarios=4
long_chat_summary_completed_scenarios=4
long_chat_summary_all_finish_reasons_present=true
long_chat_summary_all_usage_accounting_valid=true
long_chat_summary_all_token_limit_status_present=true
long_chat_summary_any_token_limit_hit=false
long_chat_summary_all_follow_up_turns_use_generated_context=true
long_chat_summary_all_generated_context_identities_match_previous_response=true
long_chat_summary_all_timing_present=true
long_chat_summary_rss_required=true
long_chat_summary_all_rss_present=true
long_chat_summary_required_models=qwen2.5-0.5b-q4_k_m
long_chat_summary_required_models_present=true
long_chat_summary_required_token_lengths=64
long_chat_summary_required_token_lengths_present=true
long_chat_summary_run_complete=true
```

## Interpretation

This validates explicit stop behavior on a bounded local Qwen2.5-0.5B slice.
The run did not hit the requested 64-token completion cap; each turn stopped
after 20 completion tokens with `finish_reason=stop`.

This is not EOS behavior proof and it is not full Tier 1 stop closure. It uses
one local model and a small deterministic prompt. The full gate still needs
stop/EOS coverage in the broader required model and token-length matrix.

## Next Step

Add an EOS-specific proof shape or mark EOS as not yet directly controllable
for local GGUF runs, then broaden required stop coverage across the remaining
Tier 1 model artifacts.
