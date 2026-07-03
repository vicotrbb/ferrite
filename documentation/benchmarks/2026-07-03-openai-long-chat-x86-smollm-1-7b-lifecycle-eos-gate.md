# Benchmark: x86_64 SmolLM2 1.7B Lifecycle Long-Chat EOS Gate

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Run the current lifecycle-instrumented OpenAI-compatible long-chat gate on a
bounded x86_64 Kubernetes pod for SmolLM2 1.7B Q4_K_M.

This closes the local-only gap for the natural tokenizer-EOS proof shape:

- natural tokenizer EOS maps to OpenAI `finish_reason=stop`;
- generated assistant content is carried into repeated follow-up turns;
- generated follow-up turns must use cached prompt tokens;
- token IDs, latency fields, RSS samples, error recovery, and disconnect
  recovery must be present;
- the integrated summary must report `long_chat_summary_run_complete=true`.

## Environment

- Ferrite commit: `02c196d`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-lifecycle-eos-smollm17`
- Node: `homelab-01`
- Pod IP: `10.42.248.206`
- Container image: `rust:1.96-bookworm`
- Architecture: `x86_64`
- CPU request: `500m`
- CPU limit: `2`
- Memory request: `2Gi`
- Memory limit: `6Gi`
- Ephemeral-storage request: `6Gi`
- Ephemeral-storage limit: `10Gi`
- Workspace size after source and model copy: `1013M`
- Proof directory:
  `target/proof/x86-lifecycle-smollm17-eos-2026-07-03/`

Both staging nodes were Ready before and after the run. The proof pod was
deleted after artifact collection. A final
`kubectl --context staging get pod ferrite-avx2-lifecycle-eos-smollm17 --ignore-not-found`
returned no pod output.

## Model

- Model: `SmolLM2-1.7B-Instruct-Q4_K_M`
- Pod path: `target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf`
- SHA256:
  `77665ea4815999596525c636fbeb56ba8b080b46ae85efef4f0d986a139834d7`

## Binaries

The binaries were built inside the amd64 pod. `file` reported both as
`ELF 64-bit LSB pie executable, x86-64`.

- `target/release/ferrite-server` SHA256:
  `c6e52e0858d8676d54636c0ef004e3b17b6f9b2f03890a86fc5ca97d462b3bac`
- `target/release/ferrite-openai-long-chat-gate` SHA256:
  `74e495ab2cf63aa2d18899498ead0ce53c677d3b92618109eba28e79e9a1386c`

Build result:

```text
Finished `release` profile [optimized] target(s) in 44.62s
```

## Server

```sh
./target/release/ferrite-server \
  --bind 127.0.0.1:18190 \
  --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf \
  --model-id SmolLM2-1.7B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 16 \
  --hard-max-tokens 64 \
  --inference-wait-ms 120000 \
  --experimental-prefix-cache
```

Readiness:

```text
GET /health -> {"status":"ok","ready":true,"model":"SmolLM2-1.7B-Instruct-Q4_K_M"}
```

Server PID for RSS sampling: `1665`.

## Gate

```sh
./target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --addr 127.0.0.1:18190 \
  --api-key local-secret \
  --models SmolLM2-1.7B-Instruct-Q4_K_M \
  --token-lengths 16 \
  --turns 4 \
  --probe-max-tokens 16 \
  --rss-pid 1665 \
  --prompt 'Question: What is the capital of France? Answer only with the city name.' \
  --assistant-context 'Paris.' \
  --follow-up 'Question: What is the capital of France? Answer only with the city name.' \
  --expect-finish-reason stop \
  --prompt-cache-key x86-smollm17-lifecycle-eos-long-chat-2026-07-03 \
  --prompt-cache-trace \
  --require-cached-follow-ups \
  --disconnect-reconnect-timeout-ms 120000 \
  --proof-log target/proof/x86-lifecycle-smollm17-eos-2026-07-03/long-chat-eos.log \
  --proof-exit-code target/proof/x86-lifecycle-smollm17-eos-2026-07-03/long-chat-eos.exit
```

The command exited `0`.

## Artifacts

| Artifact | Lines / bytes | SHA256 |
| --- | ---: | --- |
| `target/proof/x86-lifecycle-smollm17-eos-2026-07-03/long-chat-eos.log` | 214 lines | `b1794198d3b6747c8966acb9fa45474e3699c5067a3b31648dd549932d58f642` |
| `target/proof/x86-lifecycle-smollm17-eos-2026-07-03/long-chat-eos.exit` | 2 bytes | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |
| `target/proof/x86-lifecycle-smollm17-eos-2026-07-03/server.log` | 7 lines | `e339b75cd03e440fabb5e8661760e89e406cdc7ffd2196ff2002ffa36173934d` |
| `target/proof/x86-lifecycle-smollm17-eos-2026-07-03/server-ps-after.txt` | 2 lines | `cf4a35dbb89c6fa883ede3fe130ec515772eac21d22575f08da159158df46452` |
| `target/proof/x86-lifecycle-smollm17-eos-2026-07-03/sha256sums.txt` | 8 lines | `fc171bc028a0993d008d208a7b1565bd93655c89084410c9f0a4d24dac09587d` |

## Probe Results

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_error_probe_reconnect_generated_event=true
long_chat_error_probe_reconnect_started_new_generation=true
long_chat_error_probe_max_tokens=16
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
long_chat_disconnect_probe_reconnect_generated_event=true
long_chat_disconnect_probe_reconnect_started_new_generation=true
long_chat_disconnect_probe_max_tokens=16
```

## Scenario Results

| Turn | Context | Finish | Prompt tokens | Cached tokens | Cache lookup | Completion tokens | TTFT ms | Decode tok/s | Stream tok/s | RSS idle |
| ---: | --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| 1 | seed | stop | 48 | 0 | miss | 2 | 39722 | 1.186451 | 0.049303 | 1127350272 |
| 2 | generated | stop | 46 | 22 | shared_prefix_hit | 2 | 20086 | 1.216019 | 0.095655 | 1151217664 |
| 3 | generated | stop | 46 | 46 | exact_hit | 2 | 35 | 1.150590 | 2.210775 | 1151545344 |
| 4 | generated | stop | 46 | 46 | exact_hit | 2 | 31 | 1.144815 | 2.209676 | 1151647744 |

Every scenario emitted one content chunk, one token-id chunk, one token ID, and:

```text
long_chat_result_streaming_all_content_chunks_have_token_ids=true
```

The server RSS after the proof was `1124784` KiB in `ps`, with elapsed runtime
`02:42`.

## Integrated Summary

```text
long_chat_summary_planned_scenarios=4
long_chat_summary_completed_scenarios=4
long_chat_summary_all_finish_reasons_present=true
long_chat_summary_all_usage_accounting_valid=true
long_chat_summary_all_token_limit_status_present=true
long_chat_summary_any_token_limit_hit=false
long_chat_summary_prompt_cache_key_present=true
long_chat_summary_cached_follow_ups_required=true
long_chat_summary_any_cached_prompt_tokens=true
long_chat_summary_generated_follow_up_turns=3
long_chat_summary_cached_generated_follow_up_turns=3
long_chat_summary_uncached_generated_follow_up_turns=0
long_chat_summary_all_generated_follow_up_turns_cached=true
long_chat_summary_generated_follow_up_context_required=true
long_chat_summary_all_follow_up_turns_use_generated_context=true
long_chat_summary_generated_context_identity_required=true
long_chat_summary_generated_context_identity_links=3
long_chat_summary_matching_generated_context_identity_links=3
long_chat_summary_all_generated_context_identity_links_present=true
long_chat_summary_all_generated_context_identities_match_previous_response=true
long_chat_summary_all_timing_present=true
long_chat_summary_streaming_token_ids_required=true
long_chat_summary_all_streaming_token_id_summaries_present=true
long_chat_summary_all_streaming_content_chunks_have_token_ids=true
long_chat_summary_rss_required=true
long_chat_summary_all_rss_present=true
long_chat_summary_error_probe_required=true
long_chat_summary_error_probe_completed=true
long_chat_summary_error_probe_reconnect_started_new_generation=true
long_chat_summary_disconnect_probe_required=true
long_chat_summary_disconnect_probe_completed=true
long_chat_summary_disconnect_probe_reconnect_started_new_generation=true
long_chat_summary_run_complete=true
```

## Server Lifecycle

The server emitted seven lifecycle lines:

```text
openai_stream_lifecycle request_id=stream-0 finish_reason=completed disconnect_point=none prompt_tokens_started=48 prompt_cancellation_polls=1200 generated_chunks=1 generated_token_ids=1 elapsed_ms=40957
openai_stream_lifecycle request_id=stream-1 finish_reason=cancelled disconnect_point=final_chunks prompt_tokens_started=0 prompt_cancellation_polls=0 generated_chunks=1 generated_token_ids=1 elapsed_ms=829
openai_stream_lifecycle request_id=stream-2 finish_reason=completed disconnect_point=none prompt_tokens_started=0 prompt_cancellation_polls=0 generated_chunks=1 generated_token_ids=1 elapsed_ms=844
openai_stream_lifecycle request_id=stream-3 finish_reason=completed disconnect_point=none prompt_tokens_started=48 prompt_cancellation_polls=1200 generated_chunks=1 generated_token_ids=1 elapsed_ms=40564
openai_stream_lifecycle request_id=stream-4 finish_reason=completed disconnect_point=none prompt_tokens_started=24 prompt_cancellation_polls=600 generated_chunks=1 generated_token_ids=1 elapsed_ms=20908
openai_stream_lifecycle request_id=stream-5 finish_reason=completed disconnect_point=none prompt_tokens_started=0 prompt_cancellation_polls=0 generated_chunks=1 generated_token_ids=1 elapsed_ms=904
openai_stream_lifecycle request_id=stream-6 finish_reason=completed disconnect_point=none prompt_tokens_started=0 prompt_cancellation_polls=0 generated_chunks=1 generated_token_ids=1 elapsed_ms=904
```

`stream-1` is the intentional disconnect-probe cancellation. The four scenario
streams completed without disconnect.

## Interpretation

This proves the current OpenAI-compatible lifecycle server can complete the
natural-EOS SmolLM2 1.7B Q4_K_M long-chat gate on x86_64 with generated-context
carry, cache requirements, token IDs, timing, RSS, and reconnect/error probes.

The x86_64 pod is substantially slower than the local macOS run for cache-miss
and shared-prefix prefill. The exact-hit rows still collapse TTFT to
millisecond scale:

- miss: `39722` ms TTFT;
- shared-prefix hit with 22 of 46 prompt tokens cached: `20086` ms TTFT;
- exact hits with all 46 prompt tokens cached: `35` ms and `31` ms TTFT.

This supports the fixed-point cache theory for a natural-EOS workload: once the
generated assistant context stabilizes and the rendered prompt token identity is
unchanged, prefill cost disappears from user-visible TTFT.

## Limits

This run does not prove:

- 256/512/1024-token SmolLM2 generated output behavior;
- high-concurrency SmolLM2 behavior;
- Qwen natural-EOS behavior;
- long-running RSS stability;
- an optimization beyond the existing exact-hit prefix-cache path.
