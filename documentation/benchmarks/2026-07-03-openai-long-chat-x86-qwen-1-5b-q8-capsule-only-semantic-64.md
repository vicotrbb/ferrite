# OpenAI Long-Chat x86_64 Qwen 1.5B Q8 Capsule-Only Semantic 64

## Scope

This run tests whether the capsule-only placement can preserve a small
structured fact, not only an arbitrary numeric anchor. It uses the proof-only
`assistant-context-only` placement added in commit `d9161d3`.

The capsule is:

```text
risk=thermal_throttling mitigation_code=reduce_batch_size owner=runtime_scheduler
```

The generated follow-up assertion requires the model to include:

```text
reduce_batch_size
```

This is still a substring assertion, but it checks recall of a named mitigation
field rather than exact repetition of the previous `state_anchor=7291` marker.

## Environment

- Date: 2026-07-03 UTC
- Commit: `414f0f0`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-semantic-capsule-qwen15-q8`
- Node: `homelab-01`
- Pod image: `rust:1.96-bookworm`
- Pod IP: `10.42.248.232`
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
  `target/proof/x86-qwen-1-5b-q8-capsule-only-semantic-64-2026-07-03/`
- Workspace size after source copy, model copy, build, and proof: `2.0G`

The first model-copy attempt hit a transient staging API exec-stream reset. The
partial pod was deleted, the pod was recreated, and the clean rerun produced
the results below.

## Server

```sh
./target/release/ferrite-server \
  --bind 127.0.0.1:18467 \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --model-id qwen2.5-1.5b-instruct-q8_0 \
  --api-key local-secret \
  --default-max-tokens 256 \
  --hard-max-tokens 512 \
  --inference-wait-ms 120000
```

Readiness:

```text
GET /health -> {"status":"ok","ready":true,"model":"qwen2.5-1.5b-instruct-q8_0"}
```

Server PID for RSS sampling: `1646`.

## Gate Command

```sh
./target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --addr 127.0.0.1:18467 \
  --api-key local-secret \
  --models qwen2.5-1.5b-instruct-q8_0 \
  --prompt "Use the Ferrite state capsule as the only source of truth for this CPU inference continuity check." \
  --assistant-context "Initial context: the semantic continuity proof starts now." \
  --follow-up "Name the mitigation_code from the Ferrite state capsule. Answer with one short sentence and include the exact mitigation_code token." \
  --expect-finish-reason length \
  --probe-max-tokens 256 \
  --generated-context-max-tokens 64 \
  --generated-context-state-capsule "risk=thermal_throttling mitigation_code=reduce_batch_size owner=runtime_scheduler" \
  --generated-context-state-capsule-placement assistant-context-only \
  --require-generated-response-contains reduce_batch_size \
  --rss-pid 1646 \
  --token-lengths 256 \
  --turns 4 \
  --proof-log target/proof/x86-qwen-1-5b-q8-capsule-only-semantic-64-2026-07-03/semantic-64.log \
  --proof-exit-code target/proof/x86-qwen-1-5b-q8-capsule-only-semantic-64-2026-07-03/semantic-64.exit
```

The command exited `0`, and `semantic-64.exit` contains `0`.

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

| Turn | Context | Context bytes | Prompt tokens | Completion tokens | Response hash | TTFT ms | Decode tok/s | Stream tok/s | RSS idle |
| ---: | --- | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| 1 | seed | 58 | 62 | 256 | `fnv64:de190a96808ed404` | 14467 | 4.012840 | 3.283817 | 1941192704 |
| 2 | generated | 104 | 74 | 256 | `fnv64:7477d5f93ba8199e` | 17250 | 3.997178 | 3.161287 | 1941848064 |
| 3 | generated | 104 | 74 | 256 | `fnv64:7477d5f93ba8199e` | 17664 | 3.969708 | 3.128314 | 1941848064 |
| 4 | generated | 104 | 74 | 256 | `fnv64:7477d5f93ba8199e` | 17218 | 3.982092 | 3.153118 | 1941848064 |

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
| Assistant context bytes | 104.00 |
| Prompt tokens | 74.00 |
| TTFT ms | 17377.33 |
| Decode tok/s | 3.982993 |
| Stream tok/s | 3.147573 |

## Summary Markers

```text
long_chat_summary_completed_scenarios=4
long_chat_summary_generated_follow_up_turns=3
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
openai_stream_lifecycle request_id=stream-0 finish_reason=completed disconnect_point=none prompt_tokens_started=62 prompt_cancellation_polls=1798 generated_chunks=256 generated_token_ids=256 elapsed_ms=77872
openai_stream_lifecycle request_id=stream-1 finish_reason=cancelled disconnect_point=token_streaming prompt_tokens_started=62 prompt_cancellation_polls=1798 generated_chunks=2 generated_token_ids=2 elapsed_ms=14752
openai_stream_lifecycle request_id=stream-2 finish_reason=completed disconnect_point=none prompt_tokens_started=62 prompt_cancellation_polls=1798 generated_chunks=256 generated_token_ids=256 elapsed_ms=78680
openai_stream_lifecycle request_id=stream-3 finish_reason=completed disconnect_point=none prompt_tokens_started=62 prompt_cancellation_polls=1798 generated_chunks=256 generated_token_ids=256 elapsed_ms=78262
openai_stream_lifecycle request_id=stream-4 finish_reason=completed disconnect_point=none prompt_tokens_started=74 prompt_cancellation_polls=2146 generated_chunks=256 generated_token_ids=256 elapsed_ms=81295
openai_stream_lifecycle request_id=stream-5 finish_reason=completed disconnect_point=none prompt_tokens_started=74 prompt_cancellation_polls=2146 generated_chunks=256 generated_token_ids=256 elapsed_ms=82152
openai_stream_lifecycle request_id=stream-6 finish_reason=completed disconnect_point=none prompt_tokens_started=74 prompt_cancellation_polls=2146 generated_chunks=256 generated_token_ids=256 elapsed_ms=81506
```

`stream-1` is the intentional disconnect-probe cancellation. The four measured
scenario streams completed without disconnect.

## Memory

Pod cgroup after the run:

| Sample | Bytes |
| --- | ---: |
| Current | 2600845312 |
| Peak | 4633796608 |
| Max | 8589934592 |

The cgroup peak includes build and runtime activity, so it is not an isolated
serving-memory comparison.

## Artifacts

| Artifact | Lines / bytes | SHA256 |
| --- | ---: | --- |
| `target/proof/x86-qwen-1-5b-q8-capsule-only-semantic-64-2026-07-03/semantic-64.log` | 201 lines | `fd2ad2a376876cc3be632e46d131ef3e8c2543f9140191538aa72f0cc5a78d39` |
| `target/proof/x86-qwen-1-5b-q8-capsule-only-semantic-64-2026-07-03/semantic-64.exit` | 2 bytes | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |
| `target/proof/x86-qwen-1-5b-q8-capsule-only-semantic-64-2026-07-03/server.log` | 7 lines | `1c0550505a6bf6c525ab02fb029ca37d0a0f0e438376d7a943771ec3a2c787f5` |
| `target/proof/x86-qwen-1-5b-q8-capsule-only-semantic-64-2026-07-03/cgroup-memory-after.txt` | 3 lines | `95c2f2be9c646f1bc97f505f7346d76b91f3c518fdeb46c1ca7d0f087e7b1371` |

## Theory Read

This is a positive semantic-continuity slice. The model preserved
`reduce_batch_size` across turns 2-4 from a capsule-only context, and turns 2-4
all produced the same generated response hash:

```text
fnv64:7477d5f93ba8199e
```

The generated follow-up prompt averaged 74 tokens and `17377.33` ms TTFT. This
is lower than the prior `state_anchor=7291` capsule-only run, which averaged
80 prompt tokens and `18775.33` ms TTFT, because this prompt/capsule shape used
fewer rendered prompt tokens despite a longer capsule byte string.

## Limits

This run does not prove:

- general semantic continuity;
- full generated-context identity preservation;
- prompt-cache behavior, because no `prompt_cache_key` was used;
- 512-token or 1024-token completion budgets;
- a 6Gi memory envelope for Qwen 1.5B Q8;
- production serving policy for hidden or explicit state capsules.

## Cleanup

The server was stopped, the temporary pod was deleted, and a final
`kubectl --context staging get pod ferrite-avx2-semantic-capsule-qwen15-q8 --ignore-not-found`
returned no pod output. Both staging nodes were Ready after cleanup.
