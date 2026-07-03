# Benchmark: Local SmolLM2 1.7B EOS Finish Source 16

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Rerun the existing SmolLM2-1.7B EOS-sensitive long-chat shape after adding
finish-source observability. This validates that natural tokenizer EOS is now
reported as `long_chat_result_finish_source=eos` and can satisfy
`--require-finish-sources eos` in the long-chat gate.

## Environment

- Ferrite runtime code commit: `76d04adfb723a82f719786ae003b3e62bb7f43b3`
- Host: local macOS workspace
- Host architecture: `arm64`
- CPU: `Apple M1 Pro`
- Build mode: `release`
- Server: `127.0.0.1:18238`
- Model: `SmolLM2-1.7B-Instruct-Q4_K_M`
- Model path: `target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf`
- Proof directory: `target/proof/local-smollm17-eos-source-16-2026-07-03/`
- Server binary SHA256:
  `8b5fe2e682195863e0a79a65f5695d21ee0383de0f6723857bbf76ba61e639a5`
- Gate binary SHA256:
  `e78699fc3f4e5274c63d7105b1c2c31d1edf5744346d5208ecaffa5a1f533f8e`
- Model SHA256:
  `77665ea4815999596525c636fbeb56ba8b080b46ae85efef4f0d986a139834d7`

The local server was stopped after the run. A bind-specific listener check
returned `listener_present=false` for `127.0.0.1:18238`.

## Server

```sh
RUST_LOG=info target/release/ferrite-server \
  --bind 127.0.0.1:18238 \
  --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf \
  --model-id SmolLM2-1.7B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 16 \
  --hard-max-tokens 64 \
  --inference-wait-ms 60000 \
  --experimental-prefix-cache
```

Readiness:

```text
GET /health -> {"status":"ok","ready":true,"model":"SmolLM2-1.7B-Instruct-Q4_K_M"}
```

## Gate Command

```sh
target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --addr 127.0.0.1:18238 \
  --api-key local-secret \
  --models SmolLM2-1.7B-Instruct-Q4_K_M \
  --require-models SmolLM2-1.7B-Instruct-Q4_K_M \
  --prompt 'Question: What is the capital of France? Answer only with the city name.' \
  --assistant-context 'Paris.' \
  --follow-up 'Question: What is the capital of France? Answer only with the city name.' \
  --expect-finish-reason stop \
  --require-finish-sources eos \
  --token-lengths 16 \
  --require-token-lengths 16 \
  --turns 4 \
  --probe-max-tokens 16 \
  --rss-pid <server-pid> \
  --prompt-cache-key long-chat:eos-answer-only-16-source \
  --require-cached-follow-ups \
  --proof-log target/proof/local-smollm17-eos-source-16-2026-07-03/long-chat.log \
  --proof-exit-code target/proof/local-smollm17-eos-source-16-2026-07-03/long-chat.exit
```

## Artifacts

| Artifact | Lines | SHA256 |
| --- | ---: | --- |
| `target/proof/local-smollm17-eos-source-16-2026-07-03/long-chat.log` | 221 | `f5bbee7d594f34cdcc582ccf28a659d8d8a35c81c6f57feeb84050929d2623fa` |
| `target/proof/local-smollm17-eos-source-16-2026-07-03/gate.stdout` | 221 | `f5bbee7d594f34cdcc582ccf28a659d8d8a35c81c6f57feeb84050929d2623fa` |
| `target/proof/local-smollm17-eos-source-16-2026-07-03/gate.stderr` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `target/proof/local-smollm17-eos-source-16-2026-07-03/server.log` | 7 | `e93e2267a26699c5c0c4210e55068ed9c212b7886bf1b1e5cb8fd90e4b9cb51b` |
| `target/proof/local-smollm17-eos-source-16-2026-07-03/server.stdout` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `target/proof/local-smollm17-eos-source-16-2026-07-03/health.json` | 0 | `4e6d2623540d7382cd2a861357a30be3863ed52ed378e985d0646eff5f9f4fe8` |
| `target/proof/local-smollm17-eos-source-16-2026-07-03/long-chat.exit` | 1 | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |
| `target/proof/local-smollm17-eos-source-16-2026-07-03/gate-command.exit` | 1 | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |

Both exit-code files contained `0`. The health file contains one JSON payload
without a trailing newline, so `wc -l` reports `0`.

## Probe Results

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_error_probe_reconnect_generated_event=true
long_chat_error_probe_reconnect_started_new_generation=true
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
long_chat_disconnect_probe_reconnect_generated_event=true
long_chat_disconnect_probe_reconnect_started_new_generation=true
```

## Scenario Results

| Turn | Finish | Source | Completion tokens | Cached prompt tokens | TTFT ms | Tok/s | RSS before | RSS after | RSS idle |
| ---: | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | stop | eos | 2 | 0 | 7879 | 0.248434 | 1144356864 | 1150255104 | 1150255104 |
| 2 | stop | eos | 2 | 22 | 3999 | 0.479943 | 1150255104 | 1165344768 | 1165344768 |
| 3 | stop | eos | 2 | 46 | 8 | 10.916097 | 1165344768 | 1176535040 | 1176535040 |
| 4 | stop | eos | 2 | 46 | 11 | 10.131939 | 1176535040 | 1182105600 | 1182105600 |

Every scenario reported:

```text
long_chat_result_finish_reason=stop
long_chat_result_finish_source=eos
long_chat_result_hit_token_limit=false
```

The gate also reported token IDs for every streaming content chunk:

```text
long_chat_summary_streaming_token_ids_required=true
long_chat_summary_all_streaming_content_chunks_have_token_ids=true
```

## Summary

```text
long_chat_summary_planned_scenarios=4
long_chat_summary_completed_scenarios=4
long_chat_summary_all_finish_reasons_present=true
long_chat_summary_all_usage_accounting_valid=true
long_chat_summary_all_token_limit_status_present=true
long_chat_summary_any_token_limit_hit=false
long_chat_summary_prompt_cache_key_present=true
long_chat_summary_cached_follow_ups_required=true
long_chat_summary_generated_follow_up_turns=3
long_chat_summary_cached_generated_follow_up_turns=3
long_chat_summary_all_generated_follow_up_turns_cached=true
long_chat_summary_all_follow_up_turns_use_generated_context=true
long_chat_summary_all_generated_context_identities_match_previous_response=true
long_chat_summary_all_timing_present=true
long_chat_summary_all_rss_present=true
long_chat_summary_error_probe_completed=true
long_chat_summary_error_probe_reconnect_started_new_generation=true
long_chat_summary_disconnect_probe_completed=true
long_chat_summary_disconnect_probe_reconnect_started_new_generation=true
long_chat_summary_required_models=SmolLM2-1.7B-Instruct-Q4_K_M
long_chat_summary_required_models_present=true
long_chat_summary_required_token_lengths=16
long_chat_summary_required_token_lengths_present=true
long_chat_summary_required_finish_sources=eos
long_chat_summary_required_finish_sources_present=true
long_chat_summary_run_complete=true
```

## Interpretation

This validates real local tokenizer-EOS finish-source propagation on the
OpenAI-compatible streaming chat surface for `SmolLM2-1.7B-Instruct-Q4_K_M`.
It proves four generated-context turns, cached follow-up turns, RSS sampling,
client error recovery, disconnect/reconnect recovery, terminal
`finish_reason=stop`, and terminal `finish_source=eos`.

This is still a bounded EOS slice. It does not prove EOS behavior for Qwen
models, all required Tier 1 models, the 256/512/1024 token ladder, x86_64, or
longer steady-state serving.

## Next Step

Promote this EOS finish-source shape into the full Tier 1 closure matrix and
continue looking for a reliable Qwen-specific EOS prompt or harness.
