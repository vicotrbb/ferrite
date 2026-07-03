# Benchmark: Local SmolLM2 1.7B EOS Finish Source Trace 16

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Rerun the SmolLM2-1.7B EOS finish-source proof with `--prompt-cache-trace`.
The previous current-tree finish-source run proved `finish_source=eos`, but it
did not emit prompt-cache lookup fields. This run ties natural tokenizer EOS,
generated-context identity, cached-token counts, prompt-cache lookup state,
RSS sampling, and reconnect/error probes into one bounded local proof.

## Environment

- Ferrite runtime code commit: `651fe448810265d509e5c74280bf4288233bed27`
- Host: local macOS workspace
- Host architecture: `arm64`
- CPU: `Apple M1 Pro`
- Build mode: `release`
- Server: `127.0.0.1:18239`
- Model: `SmolLM2-1.7B-Instruct-Q4_K_M`
- Model path: `target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf`
- Proof directory:
  `target/proof/local-smollm17-eos-source-trace-16-2026-07-03/`
- Server binary SHA256:
  `8b5fe2e682195863e0a79a65f5695d21ee0383de0f6723857bbf76ba61e639a5`
- Gate binary SHA256:
  `e78699fc3f4e5274c63d7105b1c2c31d1edf5744346d5208ecaffa5a1f533f8e`
- Model SHA256:
  `77665ea4815999596525c636fbeb56ba8b080b46ae85efef4f0d986a139834d7`

The local server was stopped after the run. A bind-specific listener check
returned `listener_present=false` for `127.0.0.1:18239`.

## Server

```sh
RUST_LOG=info target/release/ferrite-server \
  --bind 127.0.0.1:18239 \
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
  --addr 127.0.0.1:18239 \
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
  --prompt-cache-key long-chat:eos-answer-only-16-source-trace \
  --prompt-cache-trace \
  --require-cached-follow-ups \
  --proof-log target/proof/local-smollm17-eos-source-trace-16-2026-07-03/long-chat.log \
  --proof-exit-code target/proof/local-smollm17-eos-source-trace-16-2026-07-03/long-chat.exit
```

## Artifacts

| Artifact | Lines | SHA256 |
| --- | ---: | --- |
| `target/proof/local-smollm17-eos-source-trace-16-2026-07-03/long-chat.log` | 236 | `06b0c6296d54d6a0cf9345c7a84202e60dd560bad24982687945ba413c31448d` |
| `target/proof/local-smollm17-eos-source-trace-16-2026-07-03/gate.stdout` | 236 | `06b0c6296d54d6a0cf9345c7a84202e60dd560bad24982687945ba413c31448d` |
| `target/proof/local-smollm17-eos-source-trace-16-2026-07-03/gate.stderr` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `target/proof/local-smollm17-eos-source-trace-16-2026-07-03/server.log` | 7 | `7b33e04aaf8bf7f39a95c88da21f913ae7ab83f157c94963a15f41cfc08e91ba` |
| `target/proof/local-smollm17-eos-source-trace-16-2026-07-03/server.stdout` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `target/proof/local-smollm17-eos-source-trace-16-2026-07-03/health.json` | 0 | `4e6d2623540d7382cd2a861357a30be3863ed52ed378e985d0646eff5f9f4fe8` |
| `target/proof/local-smollm17-eos-source-trace-16-2026-07-03/long-chat.exit` | 1 | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |
| `target/proof/local-smollm17-eos-source-trace-16-2026-07-03/gate-command.exit` | 1 | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |

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

| Turn | Finish | Source | Cache lookup | Cached / prompt | TTFT ms | Tok/s | RSS idle |
| ---: | --- | --- | --- | ---: | ---: | ---: | ---: |
| 1 | stop | eos | miss | 0 / 48 | 7961 | 0.246165 | 1139998720 |
| 2 | stop | eos | shared_prefix_hit | 22 / 46 | 4014 | 0.478254 | 1159266304 |
| 3 | stop | eos | exact_hit | 46 / 46 | 6 | 10.903436 | 1183563776 |
| 4 | stop | eos | exact_hit | 46 / 46 | 10 | 10.623749 | 1190887424 |

Generated response identity stabilized after the first generated answer:

```text
turn 1 generated_response_hash=fnv64:af63c74c8601c8dd
turn 2 assistant_context_hash=fnv64:af63c74c8601c8dd
turn 2 generated_response_hash=fnv64:af63c74c8601c8dd
turn 3 assistant_context_hash=fnv64:af63c74c8601c8dd
turn 3 generated_response_hash=fnv64:af63c74c8601c8dd
turn 4 assistant_context_hash=fnv64:af63c74c8601c8dd
turn 4 generated_response_hash=fnv64:af63c74c8601c8dd
```

Prompt-cache trace fields show the transition to exact hits:

```text
turn 1 prompt_hash=fnv64:29bca34202dc5f0a shared_prefix_tokens=0
turn 2 prompt_hash=fnv64:67c5f682ed91f353 selected_entry_hash=fnv64:29bca34202dc5f0a shared_prefix_tokens=22
turn 3 prompt_hash=fnv64:67c5f682ed91f353 selected_entry_hash=fnv64:67c5f682ed91f353 shared_prefix_tokens=46
turn 4 prompt_hash=fnv64:67c5f682ed91f353 selected_entry_hash=fnv64:67c5f682ed91f353 shared_prefix_tokens=46
```

Every scenario reported:

```text
long_chat_result_finish_reason=stop
long_chat_result_finish_source=eos
long_chat_result_hit_token_limit=false
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

This strengthens the SmolLM2 EOS fixed-point cache theory. The run proves that
the same short natural-EOS generated answer creates a stable rendered prompt
after turn 2, and that turns 3 and 4 become exact prompt-cache hits with
millisecond TTFT while still terminating through tokenizer EOS.

This is not full Tier 1 closure. It covers one local model, a 16-token budget,
and one answer-only prompt contract. Qwen EOS behavior, the required
256/512/1024 ladder, all required Tier 1 models, x86_64 finish-source trace
coverage, and longer steady-state memory behavior remain open.
