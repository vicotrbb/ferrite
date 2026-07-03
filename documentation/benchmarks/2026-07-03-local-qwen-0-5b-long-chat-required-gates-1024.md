# Benchmark: Local Qwen 0.5B Long-Chat Required Gates 1024

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Exercise the hardened OpenAI long-chat gate at the 1024-token length with
required model, token-length, and probe requirements enabled. This completes
the local Qwen2.5-0.5B 256/512/1024 token-length ladder, but it is still not
full Tier 1 long-chat closure.

## Environment

- Ferrite runtime code commit: `8c1cc4f`
- Host: local macOS workspace
- Server: `127.0.0.1:18233`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Served model id: `qwen2.5-0.5b-q4_k_m`
- Proof directory:
  `target/proof/local-qwen05-long-chat-required-gates-1024-2026-07-03/`
- Server binary SHA256:
  `dec0167a646244de6392efbfe5b1549c4064dbab729de894aaa87c02c988b473`
- Gate binary SHA256:
  `7a953e710de9210b2832d61fa55dc89a8f835d5207a7e18659d9f9480ab03e97`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

The local server was stopped after the run. A final bind-specific listener
check returned no listener on `127.0.0.1:18233`.

## Server

```sh
RUST_LOG=info target/release/ferrite-server \
  --bind 127.0.0.1:18233 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id qwen2.5-0.5b-q4_k_m \
  --api-key local-secret \
  --default-max-tokens 1024 \
  --hard-max-tokens 1536 \
  --inference-wait-ms 300000 \
  --experimental-prefix-cache
```

Readiness:

```text
GET /health -> {"status":"ok","ready":true,"model":"qwen2.5-0.5b-q4_k_m"}
```

## Gate Command

```sh
target/release/ferrite-openai-long-chat-gate \
  --addr 127.0.0.1:18233 \
  --api-key local-secret \
  --execute \
  --models qwen2.5-0.5b-q4_k_m \
  --require-models qwen2.5-0.5b-q4_k_m \
  --token-lengths 1024 \
  --require-token-lengths 1024 \
  --turns 4 \
  --rss-pid <server-pid> \
  --error-probe \
  --disconnect-probe \
  --require-probes error,disconnect \
  --probe-max-tokens 64 \
  --disconnect-reconnect-timeout-ms 60000 \
  --prompt-cache-trace \
  --proof-log target/proof/local-qwen05-long-chat-required-gates-1024-2026-07-03/long-chat.log \
  --proof-exit-code target/proof/local-qwen05-long-chat-required-gates-1024-2026-07-03/long-chat.exit
```

## Artifacts

| Artifact | Lines | SHA256 |
| --- | ---: | --- |
| `target/proof/local-qwen05-long-chat-required-gates-1024-2026-07-03/long-chat.log` | 230 | `6aa287e848c9914dd0253590e056220824144ade877c5f5c4d401062b8b74588` |
| `target/proof/local-qwen05-long-chat-required-gates-1024-2026-07-03/gate.stdout` | 230 | `6aa287e848c9914dd0253590e056220824144ade877c5f5c4d401062b8b74588` |
| `target/proof/local-qwen05-long-chat-required-gates-1024-2026-07-03/gate.stderr` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `target/proof/local-qwen05-long-chat-required-gates-1024-2026-07-03/server.log` | 7 | `3d63a61700c79704642aa36a2b9757db6db6cd472bf0775317afabf31ab0bdbb` |
| `target/proof/local-qwen05-long-chat-required-gates-1024-2026-07-03/server.stdout` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `target/proof/local-qwen05-long-chat-required-gates-1024-2026-07-03/health.json` | 0 | `e3284eada962df1c75177574e65d3c528a2dcc0fb990143e5877c096413857b4` |
| `target/proof/local-qwen05-long-chat-required-gates-1024-2026-07-03/long-chat.exit` | 1 | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |
| `target/proof/local-qwen05-long-chat-required-gates-1024-2026-07-03/gate-command.exit` | 1 | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |

Both exit-code files contained `0`. The health file contains one JSON payload
without a trailing newline, so `wc -l` reports `0`.

## Probe Results

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_error_probe_reconnect_generated_event=true
long_chat_error_probe_reconnect_started_new_generation=true
long_chat_error_probe_max_tokens=64
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
long_chat_disconnect_probe_reconnect_generated_event=true
long_chat_disconnect_probe_reconnect_started_new_generation=true
long_chat_disconnect_probe_max_tokens=64
```

## Scenario Results

| Turn | Finish | Completion tokens | TTFT ms | Total ms | Tok/s | p50 ms | p95 ms | RSS before | RSS after | RSS idle |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | length | 1024 | 3 | 72984 | 14.446161 | 66 | 95 | 433897472 | 411074560 | 410796032 |
| 2 | length | 1024 | 65919 | 185284 | 5.593656 | 114 | 140 | 410796032 | 421117952 | 421117952 |
| 3 | length | 1024 | 64108 | 189715 | 5.461355 | 119 | 150 | 421117952 | 428048384 | 428048384 |
| 4 | length | 1024 | 96 | 117133 | 8.904586 | 111 | 137 | 428048384 | 415989760 | 415989760 |

All four scenarios reported `finish_reason=length`,
`long_chat_result_usage_completion_tokens=1024`, token-limit hits, streaming
token-id summaries, and RSS samples.

## Summary

```text
long_chat_summary_planned_scenarios=4
long_chat_summary_completed_scenarios=4
long_chat_summary_all_finish_reasons_present=true
long_chat_summary_all_usage_accounting_valid=true
long_chat_summary_all_token_limit_status_present=true
long_chat_summary_any_token_limit_hit=true
long_chat_summary_all_timing_present=true
long_chat_summary_all_streaming_token_id_summaries_present=true
long_chat_summary_all_streaming_content_chunks_have_token_ids=true
long_chat_summary_rss_required=true
long_chat_summary_all_rss_present=true
long_chat_summary_error_probe_completed=true
long_chat_summary_error_probe_reconnect_started_new_generation=true
long_chat_summary_disconnect_probe_completed=true
long_chat_summary_disconnect_probe_reconnect_started_new_generation=true
long_chat_summary_required_probes=error,disconnect
long_chat_summary_required_probes_completed=true
long_chat_summary_required_models=qwen2.5-0.5b-q4_k_m
long_chat_summary_required_models_present=true
long_chat_summary_required_token_lengths=1024
long_chat_summary_required_token_lengths_present=true
long_chat_summary_run_complete=true
```

Generated assistant context was carried through all follow-up turns:

```text
long_chat_summary_generated_follow_up_turns=3
long_chat_summary_all_follow_up_turns_use_generated_context=true
long_chat_summary_generated_context_identity_links=3
long_chat_summary_matching_generated_context_identity_links=3
long_chat_summary_all_generated_context_identities_match_previous_response=true
```

## Interpretation

This proves the hardened required-gate path works for a local Qwen2.5-0.5B
1024-token slice and completes the local Qwen2.5-0.5B 256/512/1024 ladder.

The run also exposes a concrete latency target for optimization: generated
follow-up turns 2 and 3 had TTFT of about 64 to 66 seconds, while turn 4 dropped
to 96 ms after an exact prompt-cache hit. That makes prompt-cache restoration
and shared-prefix reuse the next high-value theory area.

This is not full long-chat closure. It covers one local model and the
`error,disconnect` operational probe pair. The dedicated milestone still
requires the remaining Tier 1 model artifacts, queue behavior, and stop/EOS
behavior.

## Next Step

Document and test prompt-cache restoration and shared-prefix behavior theories
under `documentation/theories`, then repeat the required gate across the
remaining Tier 1 HTTP model artifacts.
