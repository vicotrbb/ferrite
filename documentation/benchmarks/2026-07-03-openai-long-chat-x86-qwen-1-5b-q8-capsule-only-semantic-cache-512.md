# OpenAI Long-Chat x86_64 Qwen 1.5B Q8 Capsule-Only Semantic Cache 512

## Scope

This run repeats the semantic capsule-only prefix-cache proof at a 512-token
completion budget. It tests whether the cache behavior observed in the
256-token lane still holds when the decode phase is twice as long.

The state capsule was:

```text
risk=thermal_throttling mitigation_code=reduce_batch_size owner=runtime_scheduler
```

The generated-response assertion required `reduce_batch_size`.

## Environment

- Date: 2026-07-03 UTC
- Local HEAD before this documentation commit: `03ab51f`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-semantic-cache512-qwen15-q8`
- Node: `homelab-01`
- Pod image: `rust:1.96-bookworm`
- Pod IP: `10.42.248.237`
- Host architecture: `x86_64`
- CPU feature evidence: `/proc/cpuinfo` included `avx2`
- Model: `Qwen2.5-1.5B-Instruct-Q8_0`
- Model id served: `qwen2.5-1.5b-instruct-q8_0`
- Model path in pod: `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
- Model SHA256:
  `d7efb072e7724d25048a4fda0a3e10b04bdef5d06b1403a1c93bd9f1240a63c8`
- Server binary SHA256:
  `82388e9ccc0d6a3cbcb6b4faaa61bdeeabf4bdc5a1ef874a5aba2df5adf6c027`
- Long-chat gate binary SHA256:
  `15c795aed3cc521d5f57a0a64af1900b80a0a4bc4f1919b030f5410f142faa21`
- Raw proof directory:
  `target/proof/x86-qwen-1-5b-q8-capsule-only-semantic-cache-512-2026-07-03/`
- Workspace size after source copy, model copy, build, and proof: `2.0G`

The gate exec stream was interrupted by transient staging API-server connection
resets after turn 1. The proof process continued inside the pod and was polled
to completion. No duplicate gate run was started.

## Server

```sh
./target/release/ferrite-server \
  --bind 127.0.0.1:18469 \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --model-id qwen2.5-1.5b-instruct-q8_0 \
  --api-key local-secret \
  --default-max-tokens 512 \
  --hard-max-tokens 1024 \
  --inference-wait-ms 180000 \
  --experimental-prefix-cache
```

Readiness:

```text
GET /health -> {"status":"ok","ready":true,"model":"qwen2.5-1.5b-instruct-q8_0"}
```

Server PID for RSS sampling: `1638`.

## Gate Command

```sh
./target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --addr 127.0.0.1:18469 \
  --api-key local-secret \
  --models qwen2.5-1.5b-instruct-q8_0 \
  --prompt "Use the Ferrite state capsule as the only source of truth for this CPU inference continuity cache check." \
  --assistant-context "Initial context: the semantic cache continuity proof starts now." \
  --follow-up "Name the mitigation_code from the Ferrite state capsule. Answer with one short sentence and include the exact mitigation_code token." \
  --expect-finish-reason length \
  --probe-max-tokens 512 \
  --generated-context-max-tokens 64 \
  --generated-context-state-capsule "risk=thermal_throttling mitigation_code=reduce_batch_size owner=runtime_scheduler" \
  --generated-context-state-capsule-placement assistant-context-only \
  --require-generated-response-contains reduce_batch_size \
  --prompt-cache-key ferrite:qwen15:q8:semantic-capsule-only:cache:512:2026-07-03 \
  --prompt-cache-trace \
  --require-cached-follow-ups \
  --rss-pid 1638 \
  --token-lengths 512 \
  --turns 4 \
  --proof-log target/proof/x86-qwen-1-5b-q8-capsule-only-semantic-cache-512-2026-07-03/semantic-cache-512.log \
  --proof-exit-code target/proof/x86-qwen-1-5b-q8-capsule-only-semantic-cache-512-2026-07-03/semantic-cache-512.exit
```

The command exited `0`, and `semantic-cache-512.exit` contains `0`.

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

| Turn | Context | Prompt tokens | Cached prompt tokens | Cache lookup | Response hash | TTFT ms | Decode tok/s | Stream tok/s |
| ---: | --- | ---: | ---: | --- | --- | ---: | ---: | ---: |
| 1 | seed | 64 | 0 | miss | `fnv64:1c79f097a6549341` | 15032 | 3.711816 | 3.353584 |
| 2 | generated | 75 | 24 | shared_prefix_hit | `fnv64:9f2061894cdbfefd` | 12087 | 3.503914 | 3.242530 |
| 3 | generated | 75 | 75 | exact_hit | `fnv64:9f2061894cdbfefd` | 80 | 3.724392 | 3.729483 |
| 4 | generated | 75 | 75 | exact_hit | `fnv64:9f2061894cdbfefd` | 80 | 3.731860 | 3.736947 |

Every scenario reported:

```text
long_chat_result_finish_reason=length
long_chat_result_hit_token_limit=true
long_chat_result_streaming_content_chunks=512
long_chat_result_streaming_token_id_chunks=512
long_chat_result_streaming_token_ids=512
long_chat_result_streaming_all_content_chunks_have_token_ids=true
```

The generated follow-up average was:

| Metric | Average |
| --- | ---: |
| Prompt tokens | 75.00 |
| Cached prompt tokens | 58.00 |
| TTFT ms | 4082.33 |
| Decode tok/s | 3.653389 |
| Stream tok/s | 3.569653 |

## Cache Comparison

Compared with the 256-token semantic capsule-only cache run:

| Metric | 256 cache | 512 cache | Read |
| --- | ---: | ---: | --- |
| Generated follow-up prompt tokens | 75.00 | 75.00 | unchanged |
| Generated follow-up cached prompt tokens | 58.00 | 58.00 | unchanged |
| Generated follow-up TTFT ms | 4170.67 | 4082.33 | comparable |
| Generated follow-up decode tok/s | 3.973448 | 3.653389 | slower long decode |
| Generated follow-up stream tok/s | 3.770284 | 3.569653 | slower long stream |

The prefix-cache effect survived the longer output budget. Turn 3 and turn 4
were exact hits with 75 cached prompt tokens and `80` ms TTFT. The remaining
wall-clock cost was decode time for the 512-token completions.

## Summary Markers

```text
long_chat_summary_completed_scenarios=4
long_chat_summary_prompt_cache_key_present=true
long_chat_summary_cached_follow_ups_required=true
long_chat_summary_any_cached_prompt_tokens=true
long_chat_summary_generated_follow_up_turns=3
long_chat_summary_cached_generated_follow_up_turns=3
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
openai_stream_lifecycle request_id=stream-0 finish_reason=completed disconnect_point=none prompt_tokens_started=64 prompt_cancellation_polls=1856 generated_chunks=512 generated_token_ids=512 elapsed_ms=152284
openai_stream_lifecycle request_id=stream-1 finish_reason=cancelled disconnect_point=token_streaming prompt_tokens_started=0 prompt_cancellation_polls=0 generated_chunks=2 generated_token_ids=2 elapsed_ms=314
openai_stream_lifecycle request_id=stream-2 finish_reason=completed disconnect_point=none prompt_tokens_started=0 prompt_cancellation_polls=0 generated_chunks=512 generated_token_ids=512 elapsed_ms=136974
openai_stream_lifecycle request_id=stream-3 finish_reason=completed disconnect_point=none prompt_tokens_started=64 prompt_cancellation_polls=1856 generated_chunks=512 generated_token_ids=512 elapsed_ms=152970
openai_stream_lifecycle request_id=stream-4 finish_reason=completed disconnect_point=none prompt_tokens_started=51 prompt_cancellation_polls=1479 generated_chunks=512 generated_token_ids=512 elapsed_ms=158209
openai_stream_lifecycle request_id=stream-5 finish_reason=completed disconnect_point=none prompt_tokens_started=0 prompt_cancellation_polls=0 generated_chunks=512 generated_token_ids=512 elapsed_ms=137552
openai_stream_lifecycle request_id=stream-6 finish_reason=completed disconnect_point=none prompt_tokens_started=0 prompt_cancellation_polls=0 generated_chunks=512 generated_token_ids=512 elapsed_ms=137277
```

`stream-1` is the intentional disconnect-probe cancellation. The four measured
scenario streams completed without disconnect.

## Memory

Pod cgroup after the run:

| Sample | Bytes |
| --- | ---: |
| Current | 3257856000 |
| Peak | 5218082816 |
| Max | 8589934592 |

The cgroup peak includes build and runtime activity, so it is not an isolated
serving-memory comparison.

## Artifacts

| Artifact | Lines / bytes | SHA256 |
| --- | ---: | --- |
| `target/proof/x86-qwen-1-5b-q8-capsule-only-semantic-cache-512-2026-07-03/semantic-cache-512.log` | 218 lines | `5485b728054e2577259c3fa550169c15b016ce1fb6930752367f271e91125d18` |
| `target/proof/x86-qwen-1-5b-q8-capsule-only-semantic-cache-512-2026-07-03/semantic-cache-512.exit` | 2 bytes | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |
| `target/proof/x86-qwen-1-5b-q8-capsule-only-semantic-cache-512-2026-07-03/server.log` | 7 lines | `26ccb5790ce95606a48a95fe12d62b29416e4aece6a973d86f59ab250ca4a8d0` |
| `target/proof/x86-qwen-1-5b-q8-capsule-only-semantic-cache-512-2026-07-03/cgroup-memory-after.txt` | 3 lines | `6149c036a3072d56fba3f09a2f915f71b5f738d9cad10001683ae3f122ff0479` |

## Theory Read

This is a positive scale-up of the semantic capsule-only cache theory from 256
to 512 completion tokens. Prompt shape, cached prompt tokens, and exact-hit
behavior stayed stable. Longer outputs mostly increased decode wall time and
memory pressure, not prefill latency.

The result does not prove 1024-token stability, multi-client cache behavior, or
production serving policy for state capsules.

## Cleanup

The server was stopped, the temporary pod was deleted, and a final
`kubectl --context staging get pod ferrite-avx2-semantic-cache512-qwen15-q8 --ignore-not-found`
returned no pod output. `kubectl --context staging get --raw=/readyz` returned
`ok`, and both staging nodes were Ready after cleanup.
