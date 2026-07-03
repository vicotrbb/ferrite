# OpenAI Long-Chat x86_64 Qwen 1.5B Q8 Capsule-Only 64

## Scope

This run tests the proof-only capsule-only placement added in commit
`d9161d3`. It keeps the same Qwen2.5-1.5B Q8_0 x86_64 setup, 64-token
generated-context window, 256-token completion budget, and short
`state_anchor=7291` capsule from the prior follow-up placement run, but uses:

```text
--generated-context-state-capsule-placement assistant-context-only
```

On generated follow-up turns, the assistant context is only the state capsule.
Retained generated assistant prose is omitted.

The goal is to test whether the capsule alone preserves the continuity anchor
while reducing prompt cost and time to first token. This is a proof harness
experiment, not a serving default.

## Environment

- Date: 2026-07-03 UTC
- Commit: `d9161d3`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-capsule-only-qwen15-q8`
- Node: `homelab-01`
- Pod image: `rust:1.96-bookworm`
- Pod IP: `10.42.248.192`
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
  `target/proof/x86-qwen-1-5b-q8-capsule-only-64-2026-07-03/`
- Workspace size after source copy, model copy, build, and proof: `2.0G`

## Server

```sh
./target/release/ferrite-server \
  --bind 127.0.0.1:18466 \
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
  --addr 127.0.0.1:18466 \
  --api-key local-secret \
  --models qwen2.5-1.5b-instruct-q8_0 \
  --prompt "Use the assistant message and any Ferrite state capsule as the source of the continuity anchor for this CPU inference test." \
  --assistant-context "Initial context: CPU inference continuity proof starts now." \
  --follow-up "Write one short CPU inference risk. Copy the state_anchor value from the Ferrite state capsule if present, and end with exactly that value. Do not invent a new anchor." \
  --expect-finish-reason length \
  --probe-max-tokens 256 \
  --generated-context-max-tokens 64 \
  --generated-context-state-capsule "state_anchor=7291" \
  --generated-context-state-capsule-placement assistant-context-only \
  --require-generated-response-contains 7291 \
  --rss-pid 1646 \
  --token-lengths 256 \
  --turns 4 \
  --proof-log target/proof/x86-qwen-1-5b-q8-capsule-only-64-2026-07-03/capsule-only-64.log \
  --proof-exit-code target/proof/x86-qwen-1-5b-q8-capsule-only-64-2026-07-03/capsule-only-64.exit
```

The command exited `0`, and `capsule-only-64.exit` contains `0`.

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
| 1 | seed | 59 | 77 | 256 | `fnv64:d6a895e3f7b3cb34` | 17892 | 3.893047 | 3.072295 | 1942523904 |
| 2 | generated | 40 | 80 | 256 | `fnv64:201ea36ecbb7d57c` | 18761 | 3.877030 | 3.030979 | 1942523904 |
| 3 | generated | 40 | 80 | 256 | `fnv64:201ea36ecbb7d57c` | 18798 | 3.952379 | 3.075263 | 1944489984 |
| 4 | generated | 40 | 80 | 256 | `fnv64:201ea36ecbb7d57c` | 18767 | 3.936128 | 3.066598 | 1945145344 |

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
| Assistant context bytes | 40.00 |
| Prompt tokens | 80.00 |
| TTFT ms | 18775.33 |
| Decode tok/s | 3.921846 |
| Stream tok/s | 3.057613 |

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

`long_chat_summary_run_complete=false` is expected for this probe. The
capsule-only mode intentionally replaces the previous generated response with a
state capsule, so generated-context identity does not match the previous full
response. This run proves the anchor-preservation theory slice, not full
generated-context identity continuity.

## Server Lifecycle

```text
openai_stream_lifecycle request_id=stream-0 finish_reason=completed disconnect_point=none prompt_tokens_started=77 prompt_cancellation_polls=2233 generated_chunks=256 generated_token_ids=256 elapsed_ms=82015
openai_stream_lifecycle request_id=stream-1 finish_reason=cancelled disconnect_point=token_streaming prompt_tokens_started=77 prompt_cancellation_polls=2233 generated_chunks=2 generated_token_ids=2 elapsed_ms=18129
openai_stream_lifecycle request_id=stream-2 finish_reason=completed disconnect_point=none prompt_tokens_started=77 prompt_cancellation_polls=2233 generated_chunks=256 generated_token_ids=256 elapsed_ms=82543
openai_stream_lifecycle request_id=stream-3 finish_reason=completed disconnect_point=none prompt_tokens_started=77 prompt_cancellation_polls=2233 generated_chunks=256 generated_token_ids=256 elapsed_ms=83650
openai_stream_lifecycle request_id=stream-4 finish_reason=completed disconnect_point=none prompt_tokens_started=80 prompt_cancellation_polls=2320 generated_chunks=256 generated_token_ids=256 elapsed_ms=84790
openai_stream_lifecycle request_id=stream-5 finish_reason=completed disconnect_point=none prompt_tokens_started=80 prompt_cancellation_polls=2320 generated_chunks=256 generated_token_ids=256 elapsed_ms=83569
openai_stream_lifecycle request_id=stream-6 finish_reason=completed disconnect_point=none prompt_tokens_started=80 prompt_cancellation_polls=2320 generated_chunks=256 generated_token_ids=256 elapsed_ms=83805
```

`stream-1` is the intentional disconnect-probe cancellation. The four measured
scenario streams completed without disconnect.

## Memory

Pod cgroup after the run:

| Sample | Bytes |
| --- | ---: |
| Current | 2919444480 |
| Peak | 4853075968 |
| Max | 8589934592 |

The cgroup peak includes build and runtime activity, so it is not an isolated
serving-memory comparison.

## Artifacts

| Artifact | Lines / bytes | SHA256 |
| --- | ---: | --- |
| `target/proof/x86-qwen-1-5b-q8-capsule-only-64-2026-07-03/capsule-only-64.log` | 201 lines | `ca9c39db17956f56fcff7abfbef12388ec10282752046690608ec1f0c5e32eb8` |
| `target/proof/x86-qwen-1-5b-q8-capsule-only-64-2026-07-03/capsule-only-64.exit` | 2 bytes | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |
| `target/proof/x86-qwen-1-5b-q8-capsule-only-64-2026-07-03/server.log` | 7 lines | `fbc022bac36b67faf636c6a4fdd431a725472e4dd6b30f340927d94e00e37f77` |
| `target/proof/x86-qwen-1-5b-q8-capsule-only-64-2026-07-03/cgroup-memory-after.txt` | 3 lines | `e541cb3b8b145ac2ddb40cbad71e4710bb119ba9ac2c4ef93e24557f6f787833` |

## Comparison

| Variant | Window | Placement | Prompt tokens avg | TTFT avg ms | Response identity |
| --- | ---: | --- | ---: | ---: | --- |
| JSON capsule | 64 | follow-up | 162.00 | 38759.67 | changing |
| Short capsule | 64 | follow-up | 151.00 | 36127.00 | changing |
| Short capsule | 64 | assistant-context-only | 80.00 | 18775.33 | fixed point on turns 2-4 |

Capsule-only placement preserved the `7291` anchor through turns 2-4 while
dropping generated follow-up prompt cost by 71 tokens on average and TTFT by
`17351.67` ms versus the short follow-up capsule run.

The repeated response hash on turns 2-4 is a new fixed-point signal:

```text
fnv64:201ea36ecbb7d57c
```

That suggests the generated prose was not needed to preserve this exact anchor
contract and may have been adding prompt cost and response drift for this
prompt.

## Limits

This run does not prove:

- full generated-context identity preservation;
- semantic continuity beyond exact substring containment;
- prompt-cache behavior, because no `prompt_cache_key` was used;
- 512-token or 1024-token completion budgets;
- a 6Gi memory envelope for Qwen 1.5B Q8;
- production serving policy for hidden or explicit state capsules.

## Cleanup

The server was stopped, the temporary pod was deleted, and a final
`kubectl --context staging get pod ferrite-avx2-capsule-only-qwen15-q8 --ignore-not-found`
returned no pod output. Both staging nodes were Ready after cleanup.
