# OpenAI Long-Chat x86_64 Qwen 1.5B Q8 Short Follow-Up State Capsule 64

## Scope

This run tests the short-capsule variant of the structured continuity-anchor
theory. It keeps the same Qwen2.5-1.5B Q8_0 x86_64 setup, 64-token
generated-context window, 256-token completion budget, and follow-up user
message placement from the prior JSON-capsule run, but replaces the capsule
with:

```text
state_anchor=7291
```

The goal is to test whether a shorter, less verbose capsule preserves the
anchor while reducing prompt cost and time to first token. This is a proof-only
gate mode, not a serving default.

## Environment

- Date: 2026-07-03 UTC
- Commit: `ed47c8f`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-state-capsule-short-qwen15-q8`
- Node: `homelab-01`
- Pod image: `rust:1.96-bookworm`
- Pod IP: `10.42.248.199`
- Host architecture: `x86_64`
- CPU feature evidence: `/proc/cpuinfo` included `avx2`
- Model: `Qwen2.5-1.5B-Instruct-Q8_0`
- Model id served: `qwen2.5-1.5b-instruct-q8_0`
- Model path in pod: `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
- Model SHA256:
  `d7efb072e7724d25048a4fda0a3e10b04bdef5d06b1403a1c93bd9f1240a63c8`
- Server binary SHA256:
  `c6e52e0858d8676d54636c0ef004e3b17b6f9b2f03890a86fc5ca97d462b3bac`
- Long-chat gate binary SHA256:
  `74e495ab2cf63aa2d18899498ead0ce53c677d3b92618109eba28e79e9a1386c`
- Raw proof directory:
  `target/proof/x86-qwen-1-5b-q8-state-capsule-short-followup-64-2026-07-03/`
- Workspace size after source copy, model copy, build, and proof: `2.0G`

## Server

```sh
./target/release/ferrite-server \
  --bind 127.0.0.1:18465 \
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
  --addr 127.0.0.1:18465 \
  --api-key local-secret \
  --models qwen2.5-1.5b-instruct-q8_0 \
  --prompt "Use the assistant message and any Ferrite state capsule as the source of the continuity anchor for this CPU inference test." \
  --assistant-context "Initial context: CPU inference continuity proof starts now." \
  --follow-up "Write one short CPU inference risk. Copy the state_anchor value from the Ferrite state capsule if present, and end with exactly that value. Do not invent a new anchor." \
  --expect-finish-reason length \
  --probe-max-tokens 256 \
  --generated-context-max-tokens 64 \
  --generated-context-state-capsule "state_anchor=7291" \
  --generated-context-state-capsule-placement follow-up \
  --require-generated-response-contains 7291 \
  --rss-pid 1646 \
  --token-lengths 256 \
  --turns 4 \
  --proof-log target/proof/x86-qwen-1-5b-q8-state-capsule-short-followup-64-2026-07-03/short-followup-64.log \
  --proof-exit-code target/proof/x86-qwen-1-5b-q8-state-capsule-short-followup-64-2026-07-03/short-followup-64.exit
```

The command exited `0`, and `short-followup-64.exit` contains `0`.

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

| Turn | Context | Prompt tokens | Completion tokens | Response hash | TTFT ms | Decode tok/s | Stream tok/s | RSS idle |
| ---: | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| 1 | seed | 77 | 256 | `fnv64:d6a895e3f7b3cb34` | 18079 | 3.971713 | 3.113809 | 1942343680 |
| 2 | generated | 150 | 256 | `fnv64:91ff679bc12daccc` | 35783 | 3.846805 | 2.511428 | 1946537984 |
| 3 | generated | 149 | 256 | `fnv64:c41a37983bf08af7` | 35708 | 3.864932 | 2.520958 | 1946537984 |
| 4 | generated | 154 | 256 | `fnv64:9a4dc85f55695e1f` | 36890 | 3.812104 | 2.470096 | 1946800128 |

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
| Prompt tokens | 151.00 |
| TTFT ms | 36127.00 |
| Decode tok/s | 3.841280 |
| Stream tok/s | 2.500827 |

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

`long_chat_summary_run_complete=false` is expected for this windowing probe:
`--generated-context-max-tokens 64` intentionally truncates the prior generated
response before using it as the next assistant context. The continuity-anchor
assertion passed, but full generated-context identity did not.

## Server Lifecycle

```text
openai_stream_lifecycle request_id=stream-0 finish_reason=completed disconnect_point=none prompt_tokens_started=77 prompt_cancellation_polls=2233 generated_chunks=256 generated_token_ids=256 elapsed_ms=82205
openai_stream_lifecycle request_id=stream-1 finish_reason=cancelled disconnect_point=token_streaming prompt_tokens_started=77 prompt_cancellation_polls=2233 generated_chunks=2 generated_token_ids=2 elapsed_ms=18256
openai_stream_lifecycle request_id=stream-2 finish_reason=completed disconnect_point=none prompt_tokens_started=77 prompt_cancellation_polls=2233 generated_chunks=256 generated_token_ids=256 elapsed_ms=82915
openai_stream_lifecycle request_id=stream-3 finish_reason=completed disconnect_point=none prompt_tokens_started=77 prompt_cancellation_polls=2233 generated_chunks=256 generated_token_ids=256 elapsed_ms=82535
openai_stream_lifecycle request_id=stream-4 finish_reason=completed disconnect_point=none prompt_tokens_started=150 prompt_cancellation_polls=4350 generated_chunks=256 generated_token_ids=256 elapsed_ms=102331
openai_stream_lifecycle request_id=stream-5 finish_reason=completed disconnect_point=none prompt_tokens_started=149 prompt_cancellation_polls=4321 generated_chunks=256 generated_token_ids=256 elapsed_ms=101945
openai_stream_lifecycle request_id=stream-6 finish_reason=completed disconnect_point=none prompt_tokens_started=154 prompt_cancellation_polls=4466 generated_chunks=256 generated_token_ids=256 elapsed_ms=104044
```

`stream-1` is the intentional disconnect-probe cancellation. The four measured
scenario streams completed without disconnect.

## Memory

Pod cgroup after the run:

| Sample | Bytes |
| --- | ---: |
| Current | 3049472000 |
| Peak | 4965199872 |
| Max | 8589934592 |

The cgroup peak includes build and runtime activity, so it is not an isolated
serving-memory comparison.

## Artifacts

| Artifact | Lines / bytes | SHA256 |
| --- | ---: | --- |
| `target/proof/x86-qwen-1-5b-q8-state-capsule-short-followup-64-2026-07-03/short-followup-64.log` | 201 lines | `7335f33fa0a7d64de1161d34c90e35cf04a2316e4a7afaf2cb938e8547441d58` |
| `target/proof/x86-qwen-1-5b-q8-state-capsule-short-followup-64-2026-07-03/short-followup-64.exit` | 2 bytes | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |
| `target/proof/x86-qwen-1-5b-q8-state-capsule-short-followup-64-2026-07-03/server.log` | 7 lines | `e9fb35de5e93f7ef6447906507abb14b96cbeb5051ad7175edc91aa187efeb77` |
| `target/proof/x86-qwen-1-5b-q8-state-capsule-short-followup-64-2026-07-03/cgroup-memory-after.txt` | 3 lines | `d2ae4bc893887ae1836106d8bd82936ac5c8eae59fe5389313c79bf480922300` |

## Comparison

| Variant | Window | Placement | Prompt tokens avg | TTFT avg ms | Result |
| --- | ---: | --- | ---: | ---: | --- |
| JSON capsule | 64 | follow-up | 162.00 | 38759.67 | anchor preserved, identity summary false |
| Short capsule | 64 | follow-up | 151.00 | 36127.00 | anchor preserved, identity summary false |

The short capsule preserved the `7291` anchor through turns 2-4 while reducing
generated follow-up prompt cost by 11 tokens on average and TTFT by `2632.67`
ms versus the JSON follow-up capsule run.

This strengthens the theory that compact, authoritative state anchors can
improve continuity per prompt token. It does not prove semantic recall, broader
prompt robustness, or production serving policy.

## Limits

This run does not prove:

- full generated-context identity preservation;
- prompt-cache behavior, because no `prompt_cache_key` was used;
- 512-token or 1024-token completion budgets;
- a 6Gi memory envelope for Qwen 1.5B Q8;
- semantic recall beyond exact substring containment;
- production serving policy for hidden or explicit state capsules.

## Cleanup

The server was stopped, the temporary pod was deleted, and a final
`kubectl --context staging get pod ferrite-avx2-state-capsule-short-qwen15-q8 --ignore-not-found`
returned no pod output. Both staging nodes were Ready after cleanup.
