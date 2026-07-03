# OpenAI Long-Chat x86_64 Qwen 1.5B Q8 Mixed Cache Keys 256

## Scope

This run tests Ferrite's experimental prompt prefix cache with two
OpenAI-compatible prompt-cache keys in one long-chat gate. It extends the
semantic capsule-only cache proof from a single fixed lane to two isolated
cache lanes.

The state capsule was:

```text
risk=thermal_throttling mitigation_code=reduce_batch_size owner=runtime_scheduler
```

The generated-response assertion required `reduce_batch_size`.

## Environment

- Date: 2026-07-03 UTC
- Source copied into proof pod at local HEAD: `c442442`
- Follow-up reporting fix after this proof: `1914057`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-mixed-cache-qwen15-q8`
- Node: `homelab-01`
- Pod image: `rust:1.96-bookworm`
- Pod IP: `10.42.248.215`
- Node architecture: `amd64`
- Model: `Qwen2.5-1.5B-Instruct-Q8_0`
- Model id served: `qwen2.5-1.5b-instruct-q8_0`
- Model path in pod: `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
- Model SHA256:
  `d7efb072e7724d25048a4fda0a3e10b04bdef5d06b1403a1c93bd9f1240a63c8`
- Server binary SHA256:
  `218ba13493fcc108ea172f5ef4c0fbdf3bb38aa8250496ad5c5e119908241126`
- Long-chat gate binary SHA256:
  `054ee83bd1ae9d4363fad007300c0da9ab5b45e9acfd0f884c6e2d77649696a2`
- Raw proof directory:
  `target/proof/x86-qwen-1-5b-q8-mixed-cache-256-2026-07-03/`

The proof log was produced before `1914057`, so result rows do not include the
prompt-cache key inline. Lane attribution below follows the emitted scenario
order: the first four result rows are key A, and the next four result rows are
key B. Commit `1914057` adds explicit `prompt_cache_key` output to future result
rows.

## Server

```sh
./target/release/ferrite-server \
  --bind 127.0.0.1:18471 \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --model-id qwen2.5-1.5b-instruct-q8_0 \
  --api-key local-secret \
  --default-max-tokens 256 \
  --hard-max-tokens 512 \
  --inference-wait-ms 180000 \
  --experimental-prefix-cache
```

Readiness:

```text
GET /health -> {"status":"ok","ready":true,"model":"qwen2.5-1.5b-instruct-q8_0"}
```

Server PID for RSS sampling: `1633`.

## Gate Command

```sh
./target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --addr 127.0.0.1:18471 \
  --api-key local-secret \
  --models qwen2.5-1.5b-instruct-q8_0 \
  --prompt "Use the Ferrite state capsule as the only source of truth for this CPU inference continuity cache check." \
  --assistant-context "Initial context: the mixed semantic cache continuity proof starts now." \
  --follow-up "Name the mitigation_code from the Ferrite state capsule. Answer with one short sentence and include the exact mitigation_code token." \
  --expect-finish-reason length \
  --probe-max-tokens 256 \
  --generated-context-max-tokens 64 \
  --generated-context-state-capsule "risk=thermal_throttling mitigation_code=reduce_batch_size owner=runtime_scheduler" \
  --generated-context-state-capsule-placement assistant-context-only \
  --require-generated-response-contains reduce_batch_size \
  --prompt-cache-keys ferrite:qwen15:q8:mixed-cache:a:256:2026-07-03,ferrite:qwen15:q8:mixed-cache:b:256:2026-07-03 \
  --prompt-cache-trace \
  --require-cached-follow-ups \
  --rss-pid 1633 \
  --token-lengths 256 \
  --turns 4 \
  --proof-log target/proof/x86-qwen-1-5b-q8-mixed-cache-256-2026-07-03/mixed-cache-256.log \
  --proof-exit-code target/proof/x86-qwen-1-5b-q8-mixed-cache-256-2026-07-03/mixed-cache-256.exit
```

The command exited `0`, and `mixed-cache-256.exit` contains `0`.

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

| Lane | Turn | Context | Prompt tokens | Cached prompt tokens | Cache lookup | Response hash | TTFT ms | Decode tok/s | Stream tok/s | RSS idle |
| --- | ---: | --- | ---: | ---: | --- | --- | ---: | ---: | ---: | ---: |
| A | 1 | seed | 65 | 0 | miss | `fnv64:b7caad2615bc86af` | 16253 | 4.036932 | 3.225892 | 1950945280 |
| A | 2 | generated | 75 | 24 | shared_prefix_hit | `fnv64:9114c31bdf357363` | 12019 | 4.077083 | 3.435405 | 1956057088 |
| A | 3 | generated | 75 | 75 | exact_hit | `fnv64:9114c31bdf357363` | 87 | 4.053623 | 4.063845 | 1956319232 |
| A | 4 | generated | 75 | 75 | exact_hit | `fnv64:9114c31bdf357363` | 81 | 4.063209 | 4.073787 | 1956319232 |
| B | 1 | seed | 65 | 0 | miss | `fnv64:b7caad2615bc86af` | 15139 | 4.073784 | 3.295684 | 1959989248 |
| B | 2 | generated | 75 | 24 | shared_prefix_hit | `fnv64:9114c31bdf357363` | 11908 | 4.027099 | 3.404990 | 1965625344 |
| B | 3 | generated | 75 | 75 | exact_hit | `fnv64:9114c31bdf357363` | 91 | 4.077688 | 4.087653 | 1965756416 |
| B | 4 | generated | 75 | 75 | exact_hit | `fnv64:9114c31bdf357363` | 82 | 4.067758 | 4.078271 | 1965756416 |

Every scenario reported:

```text
long_chat_result_finish_reason=length
long_chat_result_hit_token_limit=true
long_chat_result_streaming_content_chunks=256
long_chat_result_streaming_token_ids=256
long_chat_result_streaming_all_content_chunks_have_token_ids=true
```

## Isolation Signal

Lane B turn 1 was a cache miss even though it used the same prompt token hash as
lane A turn 1. That is the expected namespace isolation behavior for distinct
prompt-cache keys.

Both lanes then warmed independently:

| Lane | Cache sequence |
| --- | --- |
| A | miss -> shared_prefix_hit -> exact_hit -> exact_hit |
| B | miss -> shared_prefix_hit -> exact_hit -> exact_hit |

Generated follow-up summary:

| Metric | Value |
| --- | ---: |
| Generated follow-up turns | 6 |
| Cached generated follow-up turns | 6 |
| Uncached generated follow-up turns | 0 |
| Exact-hit generated follow-up turns | 4 |

## Summary Markers

```text
long_chat_summary_planned_scenarios=8
long_chat_summary_completed_scenarios=8
long_chat_summary_prompt_cache_key_present=true
long_chat_summary_cached_follow_ups_required=true
long_chat_summary_any_cached_prompt_tokens=true
long_chat_summary_generated_follow_up_turns=6
long_chat_summary_cached_generated_follow_up_turns=6
long_chat_summary_uncached_generated_follow_up_turns=0
long_chat_summary_all_generated_follow_up_turns_cached=true
long_chat_summary_all_follow_up_turns_use_generated_context=true
long_chat_summary_all_timing_present=true
long_chat_summary_all_streaming_content_chunks_have_token_ids=true
long_chat_summary_all_rss_present=true
long_chat_summary_error_probe_completed=true
long_chat_summary_disconnect_probe_completed=true
long_chat_summary_disconnect_probe_reconnect_started_new_generation=true
long_chat_summary_all_generated_context_identities_match_previous_response=false
long_chat_summary_run_complete=false
```

`long_chat_summary_run_complete=false` is expected for this probe.
Capsule-only placement intentionally replaces the previous generated response
with a state capsule, so full generated-context identity cannot match by
design.

## Server Lifecycle

```text
openai_stream_lifecycle request_id=stream-0 finish_reason=completed disconnect_point=none prompt_tokens_started=65 prompt_cancellation_polls=1885 generated_chunks=256 generated_token_ids=256 elapsed_ms=78009
openai_stream_lifecycle request_id=stream-1 finish_reason=cancelled disconnect_point=token_streaming prompt_tokens_started=0 prompt_cancellation_polls=0 generated_chunks=2 generated_token_ids=2 elapsed_ms=321
openai_stream_lifecycle request_id=stream-2 finish_reason=completed disconnect_point=none prompt_tokens_started=0 prompt_cancellation_polls=0 generated_chunks=256 generated_token_ids=256 elapsed_ms=64923
openai_stream_lifecycle request_id=stream-3 finish_reason=completed disconnect_point=none prompt_tokens_started=65 prompt_cancellation_polls=1885 generated_chunks=256 generated_token_ids=256 elapsed_ms=79667
openai_stream_lifecycle request_id=stream-4 finish_reason=completed disconnect_point=none prompt_tokens_started=51 prompt_cancellation_polls=1479 generated_chunks=256 generated_token_ids=256 elapsed_ms=74809
openai_stream_lifecycle request_id=stream-5 finish_reason=completed disconnect_point=none prompt_tokens_started=0 prompt_cancellation_polls=0 generated_chunks=256 generated_token_ids=256 elapsed_ms=63240
openai_stream_lifecycle request_id=stream-6 finish_reason=completed disconnect_point=none prompt_tokens_started=0 prompt_cancellation_polls=0 generated_chunks=256 generated_token_ids=256 elapsed_ms=63086
openai_stream_lifecycle request_id=stream-7 finish_reason=completed disconnect_point=none prompt_tokens_started=65 prompt_cancellation_polls=1885 generated_chunks=256 generated_token_ids=256 elapsed_ms=77980
openai_stream_lifecycle request_id=stream-8 finish_reason=completed disconnect_point=none prompt_tokens_started=51 prompt_cancellation_polls=1479 generated_chunks=256 generated_token_ids=256 elapsed_ms=75477
openai_stream_lifecycle request_id=stream-9 finish_reason=completed disconnect_point=none prompt_tokens_started=0 prompt_cancellation_polls=0 generated_chunks=256 generated_token_ids=256 elapsed_ms=62872
openai_stream_lifecycle request_id=stream-10 finish_reason=completed disconnect_point=none prompt_tokens_started=0 prompt_cancellation_polls=0 generated_chunks=256 generated_token_ids=256 elapsed_ms=63016
```

`stream-1` is the intentional disconnect-probe cancellation. The eight measured
scenario streams completed without disconnect.

## Memory

Pod cgroup after the run:

| Sample | Bytes |
| --- | ---: |
| Current | 2211835904 |
| Peak | 4177653760 |
| Max | 8589934592 |

The cgroup peak includes build and runtime activity, so it is not an isolated
serving-memory comparison.

## Artifacts

| Artifact | Lines / bytes | SHA256 |
| --- | ---: | --- |
| `target/proof/x86-qwen-1-5b-q8-mixed-cache-256-2026-07-03/mixed-cache-256.log` | 373 lines | `6765ebf88d32faee2127010027be2baef0c4d7b6735fa2c728a1a56e6a7a65e6` |
| `target/proof/x86-qwen-1-5b-q8-mixed-cache-256-2026-07-03/mixed-cache-256.exit` | 2 bytes | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |
| `target/proof/x86-qwen-1-5b-q8-mixed-cache-256-2026-07-03/server.log` | 11 lines | `a2eb66d14d62c56644d28807332d3fd5abe269244a3bc3d87f75e6363ef0d79c` |
| `target/proof/x86-qwen-1-5b-q8-mixed-cache-256-2026-07-03/cgroup-memory-after.txt` | 3 lines | `2169c73d5577a57ef61a5f30a0c82c59f33cb1a857446e47583a7be444f79639` |

## Theory Read

This is a positive mixed-key isolation proof for the semantic capsule cache
theory. The prompt-cache key separated otherwise identical prompt token hashes,
and each key warmed into exact hits independently.

This does not prove concurrent client behavior, eviction behavior, varied
follow-up wording, or a production memory policy.

## Cleanup

The server was stopped, the temporary pod was deleted, `kubectl --context
staging get --raw=/readyz` returned `ok`, and both staging nodes were Ready
after cleanup.
