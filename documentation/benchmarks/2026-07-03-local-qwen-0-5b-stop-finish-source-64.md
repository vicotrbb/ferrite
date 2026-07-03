# Benchmark: Local Qwen 0.5B Stop Finish Source 64

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Rerun the bounded local Qwen2.5-0.5B explicit stop probe after adding
finish-source observability to Ferrite's OpenAI-compatible streaming usage
path. This verifies that explicit OpenAI stop sequences are now reported as
`long_chat_result_finish_source=stop_sequence` and can satisfy the long-chat
gate's `--require-finish-sources stop_sequence` requirement.

## Environment

- Ferrite runtime code commit: `fbceed44e597b2b770e68793fdb3948e0553f589`
- Host: local macOS workspace
- Host architecture: `arm64`
- CPU: `Apple M1 Pro`
- Build mode: `release`
- Server: `127.0.0.1:18237`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Served model id: `qwen2.5-0.5b-q4_k_m`
- Model path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- Proof directory: `target/proof/local-qwen05-stop-source-64-2026-07-03/`
- Server binary SHA256:
  `8b5fe2e682195863e0a79a65f5695d21ee0383de0f6723857bbf76ba61e639a5`
- Gate binary SHA256:
  `e78699fc3f4e5274c63d7105b1c2c31d1edf5744346d5208ecaffa5a1f533f8e`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

The local server was stopped after the run. A bind-specific listener check
returned `listener_present=false` for `127.0.0.1:18237`.

## Server

```sh
RUST_LOG=info target/release/ferrite-server \
  --bind 127.0.0.1:18237 \
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
  --addr 127.0.0.1:18237 \
  --api-key local-secret \
  --execute \
  --models qwen2.5-0.5b-q4_k_m \
  --require-models qwen2.5-0.5b-q4_k_m \
  --prompt 'Reply with only FERRITE_STOP.' \
  --assistant-context 'FERRITE_STOP' \
  --follow-up 'Reply with only FERRITE_STOP.' \
  --stop FERRITE_STOP \
  --expect-finish-reason stop \
  --require-finish-sources stop_sequence \
  --token-lengths 64 \
  --require-token-lengths 64 \
  --turns 4 \
  --rss-pid <server-pid> \
  --prompt-cache-trace \
  --proof-log target/proof/local-qwen05-stop-source-64-2026-07-03/long-chat.log \
  --proof-exit-code target/proof/local-qwen05-stop-source-64-2026-07-03/long-chat.exit
```

## Artifacts

| Artifact | Lines | SHA256 |
| --- | ---: | --- |
| `target/proof/local-qwen05-stop-source-64-2026-07-03/long-chat.log` | 221 | `3c3f75ec188e039387d4cfd986dda72ba5f5247e1f39b92c3e124cde1cb17ef9` |
| `target/proof/local-qwen05-stop-source-64-2026-07-03/gate.stdout` | 221 | `3c3f75ec188e039387d4cfd986dda72ba5f5247e1f39b92c3e124cde1cb17ef9` |
| `target/proof/local-qwen05-stop-source-64-2026-07-03/gate.stderr` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `target/proof/local-qwen05-stop-source-64-2026-07-03/server.log` | 4 | `50b09b4c302a1e868e687d0d4b77dfd6e00139d7681e11ef3fb5932c2dcacab9` |
| `target/proof/local-qwen05-stop-source-64-2026-07-03/server.stdout` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `target/proof/local-qwen05-stop-source-64-2026-07-03/health.json` | 0 | `e3284eada962df1c75177574e65d3c528a2dcc0fb990143e5877c096413857b4` |
| `target/proof/local-qwen05-stop-source-64-2026-07-03/long-chat.exit` | 1 | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |
| `target/proof/local-qwen05-stop-source-64-2026-07-03/gate-command.exit` | 1 | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |

Both exit-code files contained `0`. The health file contains one JSON payload
without a trailing newline, so `wc -l` reports `0`.

## Scenario Results

| Turn | Finish | Source | Completion tokens | Cached prompt tokens | TTFT ms | Tok/s | RSS before | RSS after | RSS idle |
| ---: | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | stop | stop_sequence | 20 | 0 | 1108 | 9.871087 | 481705984 | 488570880 | 488570880 |
| 2 | stop | stop_sequence | 20 | 12 | 1161 | 9.613050 | 488570880 | 490504192 | 490504192 |
| 3 | stop | stop_sequence | 20 | 43 | 2 | 24.484229 | 490504192 | 492339200 | 492339200 |
| 4 | stop | stop_sequence | 20 | 43 | 1 | 24.238753 | 492339200 | 494600192 | 494600192 |

Every turn reported the same generated-response hash:

```text
long_chat_result_generated_response_hash=fnv64:51fcb343638dd399
```

Every turn reported:

```text
long_chat_result_finish_source=stop_sequence
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
long_chat_summary_all_follow_up_turns_use_generated_context=true
long_chat_summary_all_generated_context_identities_match_previous_response=true
long_chat_summary_all_timing_present=true
long_chat_summary_rss_required=true
long_chat_summary_all_rss_present=true
long_chat_summary_required_models=qwen2.5-0.5b-q4_k_m
long_chat_summary_required_models_present=true
long_chat_summary_required_token_lengths=64
long_chat_summary_required_token_lengths_present=true
long_chat_summary_required_finish_sources=stop_sequence
long_chat_summary_required_finish_sources_present=true
long_chat_summary_run_complete=true
```

## Interpretation

This validates the new finish-source observability for explicit stop sequences
on a real local Qwen2.5-0.5B OpenAI-compatible streaming chat run. The run did
not hit the 64-token cap; each turn stopped after 20 completion tokens with
`finish_reason=stop` and `finish_source=stop_sequence`.

This does not prove tokenizer EOS behavior and does not close the full Tier 1
long-chat gate. It covers one local model, one token length, and explicit stop
only. EOS still needs a deterministic real-model prompt or a model-specific
harness that can produce tokenizer EOS without relying on a stop string.

## Next Step

Use the same finish-source requirement across larger Tier 1 stop probes, then
separately design and document a tokenizer-EOS proof shape that can satisfy
`--require-finish-sources eos`.
