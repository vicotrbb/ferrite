# Benchmark: Local Qwen 0.5B Queue Probe 256

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Prove the new long-chat `--queue-probe` gate against a real
OpenAI-compatible Ferrite server, then run a full 256-token, 4-turn,
two-cache-key streaming chat matrix with RSS, token timing, error reconnect,
disconnect reconnect, generated-context identity, and cache trace evidence.

This proof targets queued-client behavior and repeated long-chat correctness.
It does not replace the larger Qwen2.5-1.5B Q8_0 x86_64 mixed-key proof.

## Environment

- Ferrite commit: `7bd7f6a`
- Host: local macOS workspace
- Server: `127.0.0.1:18217`
- Server PID for RSS sampling: `87498`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Served model id: `qwen2.5-0.5b-q4_k_m`
- Proof directory:
  `target/proof/local-qwen05-queue-probe-2026-07-03/`
- Server binary SHA256:
  `65533ccd173f0d7e34370c6e29ef442c28e1e55f5de1d64dee09a04a9a68acf4`
- Long-chat gate binary SHA256:
  `b5d74acb5309c0ac64ac5830fe76d781dbe23162b7f6160ac2ebb390fa754c52`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

The staging Kubernetes API refused connections during this proof attempt, so
this run used the local real-model path instead. The local server was stopped
after the run. A final bind-specific listener check returned no listener on
`127.0.0.1:18217`.

## Server

```sh
RUST_LOG=info target/release/ferrite-server \
  --bind 127.0.0.1:18217 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id qwen2.5-0.5b-q4_k_m \
  --api-key local-secret \
  --default-max-tokens 16 \
  --hard-max-tokens 256 \
  --inference-wait-ms 120000 \
  --experimental-prefix-cache
```

Readiness:

```text
GET /health -> {"status":"ok","ready":true,"model":"qwen2.5-0.5b-q4_k_m"}
```

## Gate

```sh
target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --queue-probe \
  --addr 127.0.0.1:18217 \
  --api-key local-secret \
  --models qwen2.5-0.5b-q4_k_m \
  --token-lengths 256 \
  --turns 4 \
  --prompt 'Summarize practical CPU inference engineering constraints in three compact points.' \
  --assistant-context 'Ferrite evaluates local OpenAI-compatible streaming chat under bounded CPU memory, prompt-cache behavior, reconnect behavior, and token latency evidence.' \
  --follow-up 'Continue with one additional compact engineering note and preserve the same structure.' \
  --prompt-cache-keys ferrite:qwen05:q4:queue:a:256:2026-07-03-clean,ferrite:qwen05:q4:queue:b:256:2026-07-03-clean \
  --prompt-cache-trace \
  --probe-max-tokens 64 \
  --generated-context-max-tokens 512 \
  --disconnect-reconnect-timeout-ms 120000 \
  --rss-pid 87498 \
  --proof-log target/proof/local-qwen05-queue-probe-2026-07-03/long-chat-queue-clean.log \
  --proof-exit-code target/proof/local-qwen05-queue-probe-2026-07-03/long-chat-queue-clean.exit
```

The command exited `0`.

## Artifacts

| Artifact | Lines / bytes | SHA256 |
| --- | ---: | --- |
| `target/proof/local-qwen05-queue-probe-2026-07-03/long-chat-queue-clean.log` | 383 lines | `8f778d3249e012611f96220812315900217c7fceaa7e63b18b49402099605df8` |
| `target/proof/local-qwen05-queue-probe-2026-07-03/long-chat-queue-clean.exit` | 2 bytes | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |
| `target/proof/local-qwen05-queue-probe-2026-07-03/server.log` | 26 lines | `48e4b3ac8290188d49fa263a6ca92096ea94a3c0b3213eb99f2597b2ae00cca9` |
| `target/proof/local-qwen05-queue-probe-2026-07-03/server.stdout` | 0 bytes | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

An earlier run in the same proof directory intentionally used an assistant
context state capsule and completed all request work, but the summary reported
`long_chat_summary_run_complete=false` because the capsule changed generated
context identity hashes. That failed run is retained as a negative harness
signal, not as the passing proof.

## Queue Probe

```text
long_chat_queue_probe_holder_prompt_cache_key=ferrite:qwen05:q4:queue:a:256:2026-07-03-clean
long_chat_queue_probe_contender_prompt_cache_key=ferrite:qwen05:q4:queue:b:256:2026-07-03-clean
long_chat_queue_probe_holder_started_streaming=true
long_chat_queue_probe_holder_completed=true
long_chat_queue_probe_contender_status=200
long_chat_queue_probe_contender_completed=true
long_chat_queue_probe_contender_generated_event=true
long_chat_queue_probe_contender_started_after_holder=true
long_chat_queue_probe_max_tokens=64
```

The holder stream generated content before the contender request started. The
contender then waited behind the single-inference path, returned an OpenAI SSE
`200`, emitted generated content, and completed.

## Scenario Results

| Lane | Turn | Finish | Prompt tokens | Cached tokens | Cache lookup | Shared prefix | TTFT ms | Decode tok/s | RSS before | RSS after | RSS idle |
| --- | ---: | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| A | 1 | length | 61 | 61 | exact_hit | 61 | 73 | 21.931300 | 445874176 | 449626112 | 449626112 |
| A | 2 | length | 293 | 17 | shared_prefix_hit | 17 | 12460 | 17.489314 | 449626112 | 451231744 | 436649984 |
| A | 3 | length | 293 | 18 | shared_prefix_hit | 18 | 12365 | 17.002001 | 436649984 | 414154752 | 413581312 |
| A | 4 | length | 293 | 18 | shared_prefix_hit | 18 | 12684 | 16.894939 | 413581312 | 441827328 | 441827328 |
| B | 1 | length | 61 | 61 | exact_hit | 61 | 83 | 21.836971 | 441827328 | 436666368 | 435077120 |
| B | 2 | length | 293 | 17 | shared_prefix_hit | 17 | 13041 | 17.242476 | 435077120 | 423886848 | 423854080 |
| B | 3 | length | 293 | 18 | shared_prefix_hit | 18 | 12199 | 17.267732 | 423854080 | 430522368 | 430522368 |
| B | 4 | length | 293 | 18 | shared_prefix_hit | 18 | 12686 | 17.321652 | 430522368 | 427409408 | 427409408 |

Every scenario reported 256 content chunks, 256 token-id chunks, valid usage
accounting, and `long_chat_result_hit_token_limit=true`.

## Integrated Summary

```text
long_chat_summary_planned_scenarios=8
long_chat_summary_completed_scenarios=8
long_chat_summary_all_finish_reasons_present=true
long_chat_summary_all_usage_accounting_valid=true
long_chat_summary_all_token_limit_status_present=true
long_chat_summary_any_token_limit_hit=true
long_chat_summary_any_cached_prompt_tokens=true
long_chat_summary_generated_follow_up_turns=6
long_chat_summary_cached_generated_follow_up_turns=6
long_chat_summary_uncached_generated_follow_up_turns=0
long_chat_summary_all_generated_follow_up_turns_cached=true
long_chat_summary_all_follow_up_turns_use_generated_context=true
long_chat_summary_matching_generated_context_identity_links=6
long_chat_summary_all_generated_context_identities_match_previous_response=true
long_chat_summary_all_timing_present=true
long_chat_summary_all_streaming_token_id_summaries_present=true
long_chat_summary_all_streaming_content_chunks_have_token_ids=true
long_chat_summary_all_rss_present=true
long_chat_summary_error_probe_completed=true
long_chat_summary_error_probe_reconnect_started_new_generation=true
long_chat_summary_disconnect_probe_completed=true
long_chat_summary_disconnect_probe_reconnect_started_new_generation=true
long_chat_summary_queue_probe_completed=true
long_chat_summary_queue_probe_contender_started_after_holder=true
long_chat_summary_run_complete=true
```

## Server Lifecycle

The clean run emitted 13 lifecycle lines:

- one completed error-probe reconnect stream with 64 generated token IDs;
- one cancelled disconnect-probe stream at `disconnect_point=token_streaming`;
- one completed disconnect-probe reconnect stream with 64 generated token IDs;
- two completed queue-probe streams with 64 generated token IDs each;
- eight completed 256-token long-chat scenario streams.

No long-chat scenario stream reported a disconnect.

## Interpretation

Ferrite now has real-model proof that the OpenAI-compatible long-chat gate can
start a queued second streaming client after a first client is already
generating, then complete the queued client and the normal repeated
conversation matrix with valid timing, RSS, token IDs, generated-context
identity, reconnect behavior, and summary markers.

Because the queue probe warms both prompt-cache keys before the normal matrix,
lane B turn 1 is an `exact_hit` in this local run. This proof should not be
used as evidence that a cold lane B misses after lane A warms. The separate
Qwen2.5-1.5B Q8_0 x86_64 mixed-key proof remains the cache-key isolation
evidence for cold sequential lanes.

## Remaining Work

- Repeat queued-client proof on the x86_64 staging path when Kubernetes is
  reachable.
- Repeat queue proof with the Qwen2.5-1.5B Q8_0 semantic-capsule shape.
- Add a no-cache queue baseline to quantify queue wait cost.
- Add a pure cold-lane queue variant if the queue probe needs isolation proof
  and not only queued-client completion proof.
