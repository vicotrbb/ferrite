# Benchmark: Local SmolLM2 1.7B Lifecycle Long-Chat EOS Gate

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Refresh the natural tokenizer-EOS long-chat proof on the current
lifecycle-instrumented Ferrite OpenAI-compatible server.

This run verifies a multi-turn streaming chat shape where:

- natural tokenizer EOS maps to OpenAI `finish_reason=stop`;
- generated assistant content is carried into follow-up turns;
- cached generated follow-up turns are required and observed;
- token IDs, timing, latency-per-token summaries, RSS samples, error recovery,
  and disconnect recovery are all present;
- the integrated long-chat summary reports `run_complete=true`.

## Environment

- Ferrite commit: `e77e46d`
- Host: local macOS workspace
- Server: `127.0.0.1:18209`
- Server PID for RSS sampling: `35636`
- Model: `SmolLM2-1.7B-Instruct-Q4_K_M`
- Model path: `target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf`
- Proof directory:
  `target/proof/local-smollm17-lifecycle-long-chat-eos-2026-07-03/`
- Server binary SHA256:
  `9e6458f6ca175e830b253ef77e3d8205195f5597c3d6543ddc7c3e82f9061198`
- Long-chat gate binary SHA256:
  `414541d1efc8a64c12c8b26c2a3364d89cd54cca243e0e050496046d370eb8fa`
- Model SHA256:
  `77665ea4815999596525c636fbeb56ba8b080b46ae85efef4f0d986a139834d7`

The local server was stopped after the run. A final bind-specific process check
returned no listener on `127.0.0.1:18209`.

## Server

```sh
RUST_LOG=info target/release/ferrite-server \
  --bind 127.0.0.1:18209 \
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

## Gate

```sh
target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --addr 127.0.0.1:18209 \
  --api-key local-secret \
  --models SmolLM2-1.7B-Instruct-Q4_K_M \
  --token-lengths 16 \
  --turns 4 \
  --probe-max-tokens 16 \
  --rss-pid 35636 \
  --prompt 'Question: What is the capital of France? Answer only with the city name.' \
  --assistant-context 'Paris.' \
  --follow-up 'Question: What is the capital of France? Answer only with the city name.' \
  --expect-finish-reason stop \
  --prompt-cache-key local-smollm17-lifecycle-eos-long-chat-2026-07-03 \
  --prompt-cache-trace \
  --require-cached-follow-ups \
  --disconnect-reconnect-timeout-ms 120000 \
  --proof-log target/proof/local-smollm17-lifecycle-long-chat-eos-2026-07-03/long-chat-eos.log \
  --proof-exit-code target/proof/local-smollm17-lifecycle-long-chat-eos-2026-07-03/long-chat-eos.exit
```

The command exited `0`.

## Artifacts

| Artifact | Lines / bytes | SHA256 |
| --- | ---: | --- |
| `target/proof/local-smollm17-lifecycle-long-chat-eos-2026-07-03/long-chat-eos.log` | 214 lines | `d781864a359971000679c15ebc7c8ab90c401f00d1927347b09e36279374e5ac` |
| `target/proof/local-smollm17-lifecycle-long-chat-eos-2026-07-03/long-chat-eos.exit` | 2 bytes | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |
| `target/proof/local-smollm17-lifecycle-long-chat-eos-2026-07-03/server.log` | 7 lines | `0be3df59fddec3b727ff614fd8bd622c9b8c7f34212446c6495a2b9025da7249` |
| `target/proof/local-smollm17-lifecycle-long-chat-eos-2026-07-03/server.stdout` | 0 bytes | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

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

| Turn | Context | Finish | Prompt tokens | Cached tokens | Cache lookup | Completion tokens | Hit limit | TTFT ms | Tok/s | RSS idle |
| ---: | --- | --- | ---: | ---: | --- | ---: | --- | ---: | ---: | ---: |
| 1 | seed | stop | 48 | 0 | miss | 2 | false | 7969 | 0.245930 | 1154727936 |
| 2 | generated | stop | 46 | 22 | shared_prefix_hit | 2 | false | 4067 | 0.472851 | 1173897216 |
| 3 | generated | stop | 46 | 46 | exact_hit | 2 | false | 32 | 10.096128 | 1189937152 |
| 4 | generated | stop | 46 | 46 | exact_hit | 2 | false | 31 | 8.914202 | 1183481856 |

Each scenario emitted one content chunk, one token-id chunk, one token ID, and:

```text
long_chat_result_streaming_all_content_chunks_have_token_ids=true
```

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
openai_stream_lifecycle request_id=stream-0 finish_reason=completed disconnect_point=none prompt_tokens_started=48 prompt_cancellation_polls=1200 generated_chunks=1 generated_token_ids=1 elapsed_ms=8602
openai_stream_lifecycle request_id=stream-1 finish_reason=cancelled disconnect_point=final_chunks prompt_tokens_started=0 prompt_cancellation_polls=0 generated_chunks=1 generated_token_ids=1 elapsed_ms=195
openai_stream_lifecycle request_id=stream-2 finish_reason=completed disconnect_point=none prompt_tokens_started=0 prompt_cancellation_polls=0 generated_chunks=1 generated_token_ids=1 elapsed_ms=213
openai_stream_lifecycle request_id=stream-3 finish_reason=completed disconnect_point=none prompt_tokens_started=48 prompt_cancellation_polls=1200 generated_chunks=1 generated_token_ids=1 elapsed_ms=8132
openai_stream_lifecycle request_id=stream-4 finish_reason=completed disconnect_point=none prompt_tokens_started=24 prompt_cancellation_polls=600 generated_chunks=1 generated_token_ids=1 elapsed_ms=4229
openai_stream_lifecycle request_id=stream-5 finish_reason=completed disconnect_point=none prompt_tokens_started=0 prompt_cancellation_polls=0 generated_chunks=1 generated_token_ids=1 elapsed_ms=197
openai_stream_lifecycle request_id=stream-6 finish_reason=completed disconnect_point=none prompt_tokens_started=0 prompt_cancellation_polls=0 generated_chunks=1 generated_token_ids=1 elapsed_ms=224
```

`stream-1` is the intentional disconnect-probe cancellation. The four scenario
streams completed without disconnect.

## Interpretation

Ferrite's current lifecycle-instrumented local OpenAI-compatible server now has
fresh natural-EOS long-chat proof for SmolLM2 1.7B Q4_K_M. This strengthens the
long-chat gate coverage beyond single-request EOS: generated assistant content
was reused across turns, cache reuse was required and observed, all follow-up
context identities matched the previous response, token IDs and RSS were
present, and reconnect/error probes passed.

The exact-hit rows show the expected latency collapse after the prompt cache has
the full generated-context prompt: TTFT dropped from `7969` ms and `4067` ms to
`32` ms and `31` ms on turns 3 and 4.

## Limits

This run does not prove:

- Qwen natural EOS behavior;
- x86_64 current-commit lifecycle EOS behavior;
- 256/512/1024-token length behavior for this EOS prompt;
- high-concurrency EOS behavior;
- long-running RSS stability.
