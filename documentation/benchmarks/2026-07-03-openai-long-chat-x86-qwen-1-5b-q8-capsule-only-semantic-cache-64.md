# OpenAI Long-Chat x86_64 Qwen 1.5B Q8 Capsule-Only Semantic Cache 64

## Scope

This run tests whether the semantic capsule-only fixed prompt can benefit from
Ferrite's experimental OpenAI-compatible prompt prefix cache.

It repeats the prior capsule-only semantic proof with:

```text
--experimental-prefix-cache
--prompt-cache-key ferrite:qwen15:q8:semantic-capsule-only:cache:64:2026-07-03
--prompt-cache-trace
--require-cached-follow-ups
```

The state capsule was:

```text
risk=thermal_throttling mitigation_code=reduce_batch_size owner=runtime_scheduler
```

The generated-response assertion required `reduce_batch_size`.

## Environment

- Date: 2026-07-03 UTC
- Local HEAD before this documentation commit: `172a621`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-semantic-cache-qwen15-q8`
- Node: `homelab-01`
- Pod image: `rust:1.96-bookworm`
- Pod IP: `10.42.248.203`
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
  `target/proof/x86-qwen-1-5b-q8-capsule-only-semantic-cache-64-2026-07-03/`
- Workspace size after source copy, model copy, build, and proof: `2.0G`

The first gate exec stream hit a transient Kubernetes websocket `1006` close.
The proof process continued inside the pod, wrote a valid exit file, and was
polled to completion. No duplicate gate run was started.

## Server

```sh
./target/release/ferrite-server \
  --bind 127.0.0.1:18468 \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --model-id qwen2.5-1.5b-instruct-q8_0 \
  --api-key local-secret \
  --default-max-tokens 256 \
  --hard-max-tokens 512 \
  --inference-wait-ms 120000 \
  --experimental-prefix-cache
```

Readiness:

```text
GET /health -> {"status":"ok","ready":true,"model":"qwen2.5-1.5b-instruct-q8_0"}
```

Server PID for RSS sampling: `1659`.

## Gate Command

```sh
./target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --addr 127.0.0.1:18468 \
  --api-key local-secret \
  --models qwen2.5-1.5b-instruct-q8_0 \
  --prompt "Use the Ferrite state capsule as the only source of truth for this CPU inference continuity cache check." \
  --assistant-context "Initial context: the semantic cache continuity proof starts now." \
  --follow-up "Name the mitigation_code from the Ferrite state capsule. Answer with one short sentence and include the exact mitigation_code token." \
  --expect-finish-reason length \
  --probe-max-tokens 256 \
  --generated-context-max-tokens 64 \
  --generated-context-state-capsule "risk=thermal_throttling mitigation_code=reduce_batch_size owner=runtime_scheduler" \
  --generated-context-state-capsule-placement assistant-context-only \
  --require-generated-response-contains reduce_batch_size \
  --prompt-cache-key ferrite:qwen15:q8:semantic-capsule-only:cache:64:2026-07-03 \
  --prompt-cache-trace \
  --require-cached-follow-ups \
  --rss-pid 1659 \
  --token-lengths 256 \
  --turns 4 \
  --proof-log target/proof/x86-qwen-1-5b-q8-capsule-only-semantic-cache-64-2026-07-03/semantic-cache-64.log \
  --proof-exit-code target/proof/x86-qwen-1-5b-q8-capsule-only-semantic-cache-64-2026-07-03/semantic-cache-64.exit
```

The command exited `0`, and `semantic-cache-64.exit` contains `0`.

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
| 1 | seed | 64 | 0 | miss | `fnv64:b7caad2615bc86af` | 15352 | 3.997608 | 3.237143 |
| 2 | generated | 75 | 24 | shared_prefix_hit | `fnv64:9114c31bdf357363` | 12295 | 3.984527 | 3.357533 |
| 3 | generated | 75 | 75 | exact_hit | `fnv64:9114c31bdf357363` | 123 | 3.984693 | 3.992559 |
| 4 | generated | 75 | 75 | exact_hit | `fnv64:9114c31bdf357363` | 94 | 3.951123 | 3.960759 |

Every scenario reported:

```text
long_chat_result_finish_reason=length
long_chat_result_hit_token_limit=true
long_chat_result_streaming_content_chunks=256
long_chat_result_streaming_token_id_chunks=256
long_chat_result_streaming_token_ids=256
long_chat_result_streaming_all_content_chunks_have_token_ids=true
```

The generated follow-up average was:

| Metric | Average |
| --- | ---: |
| Prompt tokens | 75.00 |
| Cached prompt tokens | 58.00 |
| TTFT ms | 4170.67 |
| Decode tok/s | 3.973448 |
| Stream tok/s | 3.770284 |

## Cache Comparison

Compared with the prior no-cache semantic capsule-only proof:

| Metric | No cache | Cache | Delta |
| --- | ---: | ---: | ---: |
| Generated follow-up prompt tokens | 74.00 | 75.00 | +1.00 |
| Generated follow-up cached prompt tokens | 0.00 | 58.00 | +58.00 |
| Generated follow-up TTFT ms | 17377.33 | 4170.67 | -13206.66 |
| Generated follow-up stream tok/s | 3.147573 | 3.770284 | +0.622711 |

The strongest signal is turns 3 and 4. Once the fixed capsule-only prompt was
fully cached, TTFT dropped to `123` ms and `94` ms while decode throughput
remained CPU-bound at roughly 4 tokens per second.

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
openai_stream_lifecycle request_id=stream-0 finish_reason=completed disconnect_point=none prompt_tokens_started=64 prompt_cancellation_polls=1856 generated_chunks=256 generated_token_ids=256 elapsed_ms=83873
openai_stream_lifecycle request_id=stream-1 finish_reason=cancelled disconnect_point=token_streaming prompt_tokens_started=0 prompt_cancellation_polls=0 generated_chunks=2 generated_token_ids=2 elapsed_ms=367
openai_stream_lifecycle request_id=stream-2 finish_reason=completed disconnect_point=none prompt_tokens_started=0 prompt_cancellation_polls=0 generated_chunks=256 generated_token_ids=256 elapsed_ms=64067
openai_stream_lifecycle request_id=stream-3 finish_reason=completed disconnect_point=none prompt_tokens_started=64 prompt_cancellation_polls=1856 generated_chunks=256 generated_token_ids=256 elapsed_ms=79390
openai_stream_lifecycle request_id=stream-4 finish_reason=completed disconnect_point=none prompt_tokens_started=51 prompt_cancellation_polls=1479 generated_chunks=256 generated_token_ids=256 elapsed_ms=76544
openai_stream_lifecycle request_id=stream-5 finish_reason=completed disconnect_point=none prompt_tokens_started=0 prompt_cancellation_polls=0 generated_chunks=256 generated_token_ids=256 elapsed_ms=64369
openai_stream_lifecycle request_id=stream-6 finish_reason=completed disconnect_point=none prompt_tokens_started=0 prompt_cancellation_polls=0 generated_chunks=256 generated_token_ids=256 elapsed_ms=64886
```

`stream-1` is the intentional disconnect-probe cancellation. The four measured
scenario streams completed without disconnect.

## Memory

Pod cgroup after the run:

| Sample | Bytes |
| --- | ---: |
| Current | 2034368512 |
| Peak | 3903156224 |
| Max | 8589934592 |

The cgroup peak includes build and runtime activity, so it is not an isolated
serving-memory comparison.

## Artifacts

| Artifact | Lines / bytes | SHA256 |
| --- | ---: | --- |
| `target/proof/x86-qwen-1-5b-q8-capsule-only-semantic-cache-64-2026-07-03/semantic-cache-64.log` | 218 lines | `01f88273c20ef98aecdba6582708e316c85a96bfe0b9280c2659ab213b4ef3d2` |
| `target/proof/x86-qwen-1-5b-q8-capsule-only-semantic-cache-64-2026-07-03/semantic-cache-64.exit` | 2 bytes | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |
| `target/proof/x86-qwen-1-5b-q8-capsule-only-semantic-cache-64-2026-07-03/server.log` | 7 lines | `3a408f7042d6681f7dab63b9793f2d9f79b963b00a1207f8e8e95b502656d093` |
| `target/proof/x86-qwen-1-5b-q8-capsule-only-semantic-cache-64-2026-07-03/cgroup-memory-after.txt` | 3 lines | `c56b819a91b92449a4acaa38e21218f7fe28e0c1ea20d5fe3d8fdce315f3ef11` |

## Theory Read

This is a positive cache proof for the capsule-only fixed-point theory. The
follow-up prompt became stable enough for an exact prompt-cache hit on turns 3
and 4, and `--require-cached-follow-ups` passed with:

```text
long_chat_summary_cached_generated_follow_up_turns=3
long_chat_summary_uncached_generated_follow_up_turns=0
```

The model preserved `reduce_batch_size` across turns 2-4 from a capsule-only
context, and turns 2-4 all produced the same generated response hash:

```text
fnv64:9114c31bdf357363
```

This does not prove that hidden state capsules should become production serving
policy. It does prove that a compact, explicit capsule-only context can create
a cacheable fixed prompt shape for this OpenAI-compatible server path.

## Limits

This run does not prove:

- general semantic continuity;
- full generated-context identity preservation;
- cache stability across different prompts or model families;
- 512-token or 1024-token completion budgets;
- multi-client cache eviction behavior;
- a 6Gi memory envelope for Qwen 1.5B Q8;
- production serving policy for hidden or explicit state capsules.

## Cleanup

The server was stopped, the temporary pod was deleted, and a final
`kubectl --context staging get pod ferrite-avx2-semantic-cache-qwen15-q8 --ignore-not-found`
returned no pod output. `kubectl --context staging get --raw=/readyz` returned
`ok`, and both staging nodes were Ready after cleanup.
