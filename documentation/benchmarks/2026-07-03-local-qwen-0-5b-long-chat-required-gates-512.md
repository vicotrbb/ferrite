# Benchmark: Local Qwen 0.5B Long-Chat Required Gates 512

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Exercise the hardened OpenAI long-chat gate at the 512-token length with
required model, token-length, and probe requirements enabled. This is a local
Qwen2.5-0.5B 512-token proof slice, not full 256/512/1024 closure.

## Environment

- Ferrite commit: `8c1cc4f`
- Host: local macOS workspace
- Server: `127.0.0.1:18232`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Served model id: `qwen2.5-0.5b-q4_k_m`
- Proof directory:
  `target/proof/local-qwen05-long-chat-required-gates-512-2026-07-03/`
- Server binary SHA256:
  `dec0167a646244de6392efbfe5b1549c4064dbab729de894aaa87c02c988b473`
- Gate binary SHA256:
  `7a953e710de9210b2832d61fa55dc89a8f835d5207a7e18659d9f9480ab03e97`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

The local server was stopped after the run. A final bind-specific listener
check returned no listener on `127.0.0.1:18232`.

## Server

```sh
RUST_LOG=info target/release/ferrite-server \
  --bind 127.0.0.1:18232 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id qwen2.5-0.5b-q4_k_m \
  --api-key local-secret \
  --default-max-tokens 512 \
  --hard-max-tokens 1024 \
  --inference-wait-ms 180000 \
  --experimental-prefix-cache
```

Readiness:

```text
GET /health -> {"status":"ok","ready":true,"model":"qwen2.5-0.5b-q4_k_m"}
```

## Gate Command

```sh
target/release/ferrite-openai-long-chat-gate \
  --addr 127.0.0.1:18232 \
  --api-key local-secret \
  --execute \
  --models qwen2.5-0.5b-q4_k_m \
  --require-models qwen2.5-0.5b-q4_k_m \
  --token-lengths 512 \
  --require-token-lengths 512 \
  --turns 4 \
  --rss-pid <server-pid> \
  --error-probe \
  --disconnect-probe \
  --require-probes error,disconnect \
  --probe-max-tokens 64 \
  --disconnect-reconnect-timeout-ms 60000 \
  --prompt-cache-trace \
  --proof-log target/proof/local-qwen05-long-chat-required-gates-512-2026-07-03/long-chat.log \
  --proof-exit-code target/proof/local-qwen05-long-chat-required-gates-512-2026-07-03/long-chat.exit
```

## Artifacts

| Artifact | Lines | SHA256 |
| --- | ---: | --- |
| `target/proof/local-qwen05-long-chat-required-gates-512-2026-07-03/long-chat.log` | 230 | `0fc4cc1ccb7225931da72a412c81437acde9d068a6b9c70707ad5448cecde256` |
| `target/proof/local-qwen05-long-chat-required-gates-512-2026-07-03/gate.stdout` | 230 | `0fc4cc1ccb7225931da72a412c81437acde9d068a6b9c70707ad5448cecde256` |
| `target/proof/local-qwen05-long-chat-required-gates-512-2026-07-03/gate.stderr` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `target/proof/local-qwen05-long-chat-required-gates-512-2026-07-03/server.log` | 7 | `29138a1d566c18cbaf91b7cb7f5c841b06412fe03f777bd89c816add3a0750f3` |
| `target/proof/local-qwen05-long-chat-required-gates-512-2026-07-03/server.stdout` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `target/proof/local-qwen05-long-chat-required-gates-512-2026-07-03/health.json` | 1 | `2b68d51958114f7e29bc03cfa4d5ad1e18f511877011a629786ebee4448f06cb` |
| `target/proof/local-qwen05-long-chat-required-gates-512-2026-07-03/long-chat.exit` | 1 | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |
| `target/proof/local-qwen05-long-chat-required-gates-512-2026-07-03/gate-command.exit` | 1 | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |

Both exit-code files contained `0`.

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
| 1 | length | 512 | 3 | 26161 | 19.609043 | 50 | 63 | 472170496 | 467501056 | 467501056 |
| 2 | length | 512 | 26409 | 65719 | 7.805916 | 76 | 90 | 467501056 | 412631040 | 411566080 |
| 3 | length | 512 | 13504 | 51880 | 9.888067 | 75 | 87 | 411566080 | 468893696 | 468566016 |
| 4 | length | 512 | 26206 | 65012 | 7.890732 | 75 | 87 | 468566016 | 426164224 | 425426944 |

All four scenarios reported `finish_reason=length`,
`long_chat_result_usage_completion_tokens=512`, token-limit hits, streaming
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
long_chat_summary_required_token_lengths=512
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
512-token slice. It adds the second local token-length rung after the 256-token
proof.

This is not full long-chat closure. It covers one local model and one token
length. The dedicated milestone still requires the full Tier 1 model set,
`1024` for this local Qwen ladder, and stop/EOS behavior.

## Next Step

Run the same required-gate shape for `1024`, then repeat across the remaining
Tier 1 HTTP model artifacts.
