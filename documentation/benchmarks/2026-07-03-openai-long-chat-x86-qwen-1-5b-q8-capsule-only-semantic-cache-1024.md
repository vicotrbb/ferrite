# OpenAI Long-Chat x86_64 Qwen 1.5B Q8 Capsule-Only Semantic Cache 1024

## Scope

This run repeats the semantic capsule-only prefix-cache proof at a 1024-token
completion budget. It tests the largest long-chat gate budget in the current
cache-scaling sequence.

The state capsule was:

```text
risk=thermal_throttling mitigation_code=reduce_batch_size owner=runtime_scheduler
```

The generated-response assertion required `reduce_batch_size`.

## Environment

- Date: 2026-07-03 UTC
- Local HEAD before this documentation commit: `b3a02ed`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-semantic-cache1024-qwen15-q8`
- Node: `homelab-01`
- Pod image: `rust:1.96-bookworm`
- Pod IP: `10.42.248.245`
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
  `target/proof/x86-qwen-1-5b-q8-capsule-only-semantic-cache-1024-2026-07-03/`
- Workspace size after source copy, model copy, build, and proof: `2.0G`

The gate exec stream was interrupted by transient staging API-server connection
resets during the run. The proof process continued inside the pod and was
polled to completion. No duplicate gate run was started.

## Server

```sh
./target/release/ferrite-server \
  --bind 127.0.0.1:18470 \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --model-id qwen2.5-1.5b-instruct-q8_0 \
  --api-key local-secret \
  --default-max-tokens 1024 \
  --hard-max-tokens 1536 \
  --inference-wait-ms 360000 \
  --experimental-prefix-cache
```

Readiness:

```text
GET /health -> {"status":"ok","ready":true,"model":"qwen2.5-1.5b-instruct-q8_0"}
```

Server PID for RSS sampling: `1637`.

## Gate Command

```sh
./target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --addr 127.0.0.1:18470 \
  --api-key local-secret \
  --models qwen2.5-1.5b-instruct-q8_0 \
  --prompt "Use the Ferrite state capsule as the only source of truth for this CPU inference continuity cache check." \
  --assistant-context "Initial context: the semantic cache continuity proof starts now." \
  --follow-up "Name the mitigation_code from the Ferrite state capsule. Answer with one short sentence and include the exact mitigation_code token." \
  --expect-finish-reason length \
  --probe-max-tokens 1024 \
  --generated-context-max-tokens 64 \
  --generated-context-state-capsule "risk=thermal_throttling mitigation_code=reduce_batch_size owner=runtime_scheduler" \
  --generated-context-state-capsule-placement assistant-context-only \
  --require-generated-response-contains reduce_batch_size \
  --prompt-cache-key ferrite:qwen15:q8:semantic-capsule-only:cache:1024:2026-07-03 \
  --prompt-cache-trace \
  --require-cached-follow-ups \
  --rss-pid 1637 \
  --token-lengths 1024 \
  --turns 4 \
  --proof-log target/proof/x86-qwen-1-5b-q8-capsule-only-semantic-cache-1024-2026-07-03/semantic-cache-1024.log \
  --proof-exit-code target/proof/x86-qwen-1-5b-q8-capsule-only-semantic-cache-1024-2026-07-03/semantic-cache-1024.exit
```

The command exited `0`, and `semantic-cache-1024.exit` contains `0`.

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
| 1 | seed | 64 | 0 | miss | `fnv64:57b4597e7d46c6ca` | 14517 | 3.190621 | 3.055524 |
| 2 | generated | 75 | 24 | shared_prefix_hit | `fnv64:13194ac0598c4307` | 12082 | 3.318422 | 3.196506 |
| 3 | generated | 75 | 75 | exact_hit | `fnv64:13194ac0598c4307` | 83 | 3.313390 | 3.315732 |
| 4 | generated | 75 | 75 | exact_hit | `fnv64:13194ac0598c4307` | 101 | 3.274822 | 3.276961 |

Every scenario reported:

```text
long_chat_result_finish_reason=length
long_chat_result_hit_token_limit=true
long_chat_result_streaming_content_chunks=1024
long_chat_result_streaming_token_id_chunks=1024
long_chat_result_streaming_token_ids=1024
long_chat_result_streaming_all_content_chunks_have_token_ids=true
```

The generated follow-up average was:

| Metric | Average |
| --- | ---: |
| Prompt tokens | 75.00 |
| Cached prompt tokens | 58.00 |
| TTFT ms | 4088.67 |
| Decode tok/s | 3.302211 |
| Stream tok/s | 3.263066 |

## Cache Comparison

Compared with the smaller semantic capsule-only cache runs:

| Metric | 256 cache | 512 cache | 1024 cache |
| --- | ---: | ---: | ---: |
| Generated follow-up prompt tokens | 75.00 | 75.00 | 75.00 |
| Generated follow-up cached prompt tokens | 58.00 | 58.00 | 58.00 |
| Generated follow-up TTFT ms | 4170.67 | 4082.33 | 4088.67 |
| Turn 3 TTFT ms | 123 | 80 | 83 |
| Turn 4 TTFT ms | 94 | 80 | 101 |
| Generated follow-up stream tok/s | 3.770284 | 3.569653 | 3.263066 |

The prefix-cache effect survived the 1024-token output budget. Prompt shape and
cache accounting stayed stable across 256, 512, and 1024 tokens. Longer output
budgets mainly reduced decode and stream throughput.

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
openai_stream_lifecycle request_id=stream-0 finish_reason=completed disconnect_point=none prompt_tokens_started=64 prompt_cancellation_polls=1856 generated_chunks=1024 generated_token_ids=1024 elapsed_ms=324924
openai_stream_lifecycle request_id=stream-1 finish_reason=cancelled disconnect_point=token_streaming prompt_tokens_started=0 prompt_cancellation_polls=0 generated_chunks=2 generated_token_ids=2 elapsed_ms=327
openai_stream_lifecycle request_id=stream-2 finish_reason=completed disconnect_point=none prompt_tokens_started=0 prompt_cancellation_polls=0 generated_chunks=1024 generated_token_ids=1024 elapsed_ms=317927
openai_stream_lifecycle request_id=stream-3 finish_reason=completed disconnect_point=none prompt_tokens_started=64 prompt_cancellation_polls=1856 generated_chunks=1024 generated_token_ids=1024 elapsed_ms=335457
openai_stream_lifecycle request_id=stream-4 finish_reason=completed disconnect_point=none prompt_tokens_started=51 prompt_cancellation_polls=1479 generated_chunks=1024 generated_token_ids=1024 elapsed_ms=320663
openai_stream_lifecycle request_id=stream-5 finish_reason=completed disconnect_point=none prompt_tokens_started=0 prompt_cancellation_polls=0 generated_chunks=1024 generated_token_ids=1024 elapsed_ms=309132
openai_stream_lifecycle request_id=stream-6 finish_reason=completed disconnect_point=none prompt_tokens_started=0 prompt_cancellation_polls=0 generated_chunks=1024 generated_token_ids=1024 elapsed_ms=312789
```

`stream-1` is the intentional disconnect-probe cancellation. The four measured
scenario streams completed without disconnect.

## Memory

Pod cgroup after the run:

| Sample | Bytes |
| --- | ---: |
| Current | 3022180352 |
| Peak | 4884017152 |
| Max | 8589934592 |

The cgroup peak includes build and runtime activity, so it is not an isolated
serving-memory comparison.

## Artifacts

| Artifact | Lines / bytes | SHA256 |
| --- | ---: | --- |
| `target/proof/x86-qwen-1-5b-q8-capsule-only-semantic-cache-1024-2026-07-03/semantic-cache-1024.log` | 218 lines | `fd995a81b70754e190e80687c5440189fc10acdef91ce3cc9b32f8c79b953482` |
| `target/proof/x86-qwen-1-5b-q8-capsule-only-semantic-cache-1024-2026-07-03/semantic-cache-1024.exit` | 2 bytes | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |
| `target/proof/x86-qwen-1-5b-q8-capsule-only-semantic-cache-1024-2026-07-03/server.log` | 7 lines | `434f93e928a0503eba165014cfe9543a388ca946cc7bed38e3115fe94b7ff0a0` |
| `target/proof/x86-qwen-1-5b-q8-capsule-only-semantic-cache-1024-2026-07-03/cgroup-memory-after.txt` | 3 lines | `cffa23ca872f143f9004ee9e3dc3f2a0bff7e2b301a380f7f25cf27db4d0e41a` |

## Theory Read

This is a positive scale-up of the semantic capsule-only cache theory from 256
and 512 to 1024 completion tokens. Cache behavior stayed stable. The model
preserved `reduce_batch_size`, all generated follow-up turns were cached, and
turns 3-4 were exact hits.

The dominant cost at 1024 tokens is decode time. Prefix caching removes repeated
prefill cost, but it does not make long CPU generation fast.

## Cleanup

The server was stopped, the temporary pod was deleted, and a final
`kubectl --context staging get pod ferrite-avx2-semantic-cache1024-qwen15-q8 --ignore-not-found`
returned no pod output. `kubectl --context staging get --raw=/readyz` returned
`ok`, and both staging nodes were Ready after cleanup.
