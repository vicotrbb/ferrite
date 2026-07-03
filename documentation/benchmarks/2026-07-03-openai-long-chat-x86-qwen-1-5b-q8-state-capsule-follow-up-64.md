# OpenAI Long-Chat x86_64 Qwen 1.5B Q8 Follow-Up State Capsule 64

## Scope

This run tests a proof-only state-capsule placement theory against Ferrite's
OpenAI-compatible HTTP server. It reruns the 64-token generated-context window
from the prior state-capsule probe, but places the capsule in the follow-up user
message instead of the assistant-context block.

The goal is narrow: determine whether follow-up placement keeps the compact
anchor `7291` alive across repeated generated-context turns. This is not a
serving default and does not change public HTTP behavior.

## Environment

- Date: 2026-07-03 UTC
- Commit: `211360a`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-state-capsule-followup-qwen15-q8`
- Node: `homelab-01`
- Pod image: `rust:1.96-bookworm`
- Pod IP: `10.42.248.250`
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
  `target/proof/x86-qwen-1-5b-q8-state-capsule-followup-64-2026-07-03/`
- Workspace size after source copy, model copy, build, and proof: `2.0G`

The first source-copy attempt hit a transient staging API readiness failure.
The partial pod was deleted, the pod was recreated, and the clean rerun produced
the results below.

## Server

```sh
./target/release/ferrite-server \
  --bind 127.0.0.1:18464 \
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

Server PID for RSS sampling: `1670`.

## Gate Command

```sh
./target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --addr 127.0.0.1:18464 \
  --api-key local-secret \
  --models qwen2.5-1.5b-instruct-q8_0 \
  --prompt "Use the assistant message and any Ferrite state capsule as the source of the continuity anchor for this CPU inference test." \
  --assistant-context "Initial context: CPU inference continuity proof starts now." \
  --follow-up "Write one short CPU inference risk. Copy the state_anchor value from the Ferrite state capsule if present, and end with exactly that value. Do not invent a new anchor." \
  --expect-finish-reason length \
  --probe-max-tokens 256 \
  --generated-context-max-tokens 64 \
  --generated-context-state-capsule '{"state_anchor":"7291","rule":"End every answer with exactly this anchor."}' \
  --generated-context-state-capsule-placement follow-up \
  --require-generated-response-contains 7291 \
  --rss-pid 1670 \
  --token-lengths 256 \
  --turns 4 \
  --proof-log target/proof/x86-qwen-1-5b-q8-state-capsule-followup-64-2026-07-03/followup-64.log \
  --proof-exit-code target/proof/x86-qwen-1-5b-q8-state-capsule-followup-64-2026-07-03/followup-64.exit
```

The command exited `0`, and `followup-64.exit` contains `0`.

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
| 1 | seed | 77 | 256 | `fnv64:d6a895e3f7b3cb34` | 17909 | 4.004087 | 3.140116 | 1943773184 |
| 2 | generated | 162 | 256 | `fnv64:63a0dc0e21c990ca` | 38502 | 3.819261 | 2.435303 | 1940377600 |
| 3 | generated | 162 | 256 | `fnv64:94ed86e3b88b0694` | 38962 | 3.846488 | 2.435622 | 1948372992 |
| 4 | generated | 162 | 256 | `fnv64:ef2ae88ffb497854` | 38815 | 3.840841 | 2.436776 | 1937965056 |

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
| Prompt tokens | 162.00 |
| TTFT ms | 38759.67 |
| Decode tok/s | 3.835530 |
| Stream tok/s | 2.435900 |

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

`long_chat_summary_run_complete=false` is expected for this specific windowing
probe because `--generated-context-max-tokens 64` intentionally truncates the
previous generated response before using it as the next assistant context. That
breaks full generated-context identity matching even when the continuity anchor
assertion passes.

## Server Lifecycle

```text
openai_stream_lifecycle request_id=stream-0 finish_reason=completed disconnect_point=none prompt_tokens_started=77 prompt_cancellation_polls=2233 generated_chunks=256 generated_token_ids=256 elapsed_ms=81833
openai_stream_lifecycle request_id=stream-1 finish_reason=cancelled disconnect_point=token_streaming prompt_tokens_started=77 prompt_cancellation_polls=2233 generated_chunks=2 generated_token_ids=2 elapsed_ms=18113
openai_stream_lifecycle request_id=stream-2 finish_reason=completed disconnect_point=none prompt_tokens_started=77 prompt_cancellation_polls=2233 generated_chunks=256 generated_token_ids=256 elapsed_ms=82435
openai_stream_lifecycle request_id=stream-3 finish_reason=completed disconnect_point=none prompt_tokens_started=77 prompt_cancellation_polls=2233 generated_chunks=256 generated_token_ids=256 elapsed_ms=81843
openai_stream_lifecycle request_id=stream-4 finish_reason=completed disconnect_point=none prompt_tokens_started=162 prompt_cancellation_polls=4698 generated_chunks=256 generated_token_ids=256 elapsed_ms=105530
openai_stream_lifecycle request_id=stream-5 finish_reason=completed disconnect_point=none prompt_tokens_started=162 prompt_cancellation_polls=4698 generated_chunks=256 generated_token_ids=256 elapsed_ms=105516
openai_stream_lifecycle request_id=stream-6 finish_reason=completed disconnect_point=none prompt_tokens_started=162 prompt_cancellation_polls=4698 generated_chunks=256 generated_token_ids=256 elapsed_ms=105467
```

`stream-1` is the intentional disconnect-probe cancellation. The four measured
scenario streams completed without disconnect.

## Memory

Pod cgroup after the run:

| Sample | Bytes |
| --- | ---: |
| Current | 3358457856 |
| Peak | 5243908096 |
| Max | 8589934592 |

## Artifacts

| Artifact | Lines / bytes | SHA256 |
| --- | ---: | --- |
| `target/proof/x86-qwen-1-5b-q8-state-capsule-followup-64-2026-07-03/followup-64.log` | 201 lines | `04e9082b714152bb120fa511299f54bc1ec8ec3ee889320e1b9be134fd658539` |
| `target/proof/x86-qwen-1-5b-q8-state-capsule-followup-64-2026-07-03/followup-64.exit` | 2 bytes | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |
| `target/proof/x86-qwen-1-5b-q8-state-capsule-followup-64-2026-07-03/server.log` | 7 lines | `57a09b8946e9a6f7824659195dc2368940a630315b057f3b13fdda0c2afc93f1` |
| `target/proof/x86-qwen-1-5b-q8-state-capsule-followup-64-2026-07-03/cgroup-memory-after.txt` | 3 lines | `22e01665225f3a009f108bb0301753b935d78aacc070ee287a6f8ef36a3a6965` |

## Theory Read

This is a positive placement signal, not a full long-chat gate closure.

Compared with the prior assistant-context placement run, the 64-token follow-up
placement completed all four measured turns and preserved the required anchor
substring. The earlier assistant-context placement failed at turn 2 for the
same 64-token generated-context window.

Follow-up placement appears to give the compact state capsule higher authority
than placing it beside retained assistant prose. It also increases prompt cost:
generated follow-up turns used 162 prompt tokens and averaged `38759.67` ms
TTFT, versus the earlier assistant-context 32-token successful run averaging
128.67 prompt tokens and `30655.00` ms TTFT.

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
`kubectl --context staging get pod ferrite-avx2-state-capsule-followup-qwen15-q8 --ignore-not-found`
returned no pod output. Both staging nodes were Ready after cleanup.
