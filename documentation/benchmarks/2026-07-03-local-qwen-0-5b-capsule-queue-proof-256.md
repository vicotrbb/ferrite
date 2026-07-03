# Benchmark: Local Qwen 0.5B Capsule Queue Proof 256

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Prove the rebuilt long-chat gate after the capsule-aware generated-context
identity fix. The run keeps an assistant-context state capsule enabled while
executing a real OpenAI-compatible 256-token, 4-turn, two-key queue proof.

This proof specifically verifies that the rendered assistant context may be
wrapped by a state capsule while the carried generated-context identity still
matches the previous generated response.

## Environment

- Ferrite commit: `52914b8`
- Host: local macOS workspace
- Server: `127.0.0.1:18218`
- Server PID for RSS sampling: `92783`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Served model id: `qwen2.5-0.5b-q4_k_m`
- Proof directory:
  `target/proof/local-qwen05-capsule-queue-proof-2026-07-03/`
- Server binary SHA256:
  `50c221c62302c644f0278c5c52ead73e68cc5247e7fe154ff3bf4702d3d6cb59`
- Long-chat gate binary SHA256:
  `14949b0f4808afe248fbdf5e5be7c20dd5011fba92c74f8c8c5084438228e2a4`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

The local server was stopped after the run. A final bind-specific listener
check returned no listener on `127.0.0.1:18218`.

## Server

```sh
RUST_LOG=info target/release/ferrite-server \
  --bind 127.0.0.1:18218 \
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
  --addr 127.0.0.1:18218 \
  --api-key local-secret \
  --models qwen2.5-0.5b-q4_k_m \
  --token-lengths 256 \
  --turns 4 \
  --prompt 'Summarize practical CPU inference engineering constraints in three compact points.' \
  --assistant-context 'Ferrite evaluates local OpenAI-compatible streaming chat under bounded CPU memory, prompt-cache behavior, reconnect behavior, and token latency evidence.' \
  --follow-up 'Continue with one additional compact engineering note and preserve the same structure.' \
  --prompt-cache-keys ferrite:qwen05:q4:capsule-queue:a:256:2026-07-03,ferrite:qwen05:q4:capsule-queue:b:256:2026-07-03 \
  --prompt-cache-trace \
  --probe-max-tokens 64 \
  --generated-context-max-tokens 512 \
  --generated-context-state-capsule 'State capsule: keep answers concise, number the points, and mention CPU, memory, and streaming reliability.' \
  --generated-context-state-capsule-placement assistant-context \
  --disconnect-reconnect-timeout-ms 120000 \
  --rss-pid 92783 \
  --proof-log target/proof/local-qwen05-capsule-queue-proof-2026-07-03/long-chat-capsule-queue.log \
  --proof-exit-code target/proof/local-qwen05-capsule-queue-proof-2026-07-03/long-chat-capsule-queue.exit
```

The command exited `0`.

## Artifacts

| Artifact | Lines / bytes | SHA256 |
| --- | ---: | --- |
| `target/proof/local-qwen05-capsule-queue-proof-2026-07-03/long-chat-capsule-queue.log` | 397 lines | `d59be4eebb326e9d3c49a100e2ca13160c50d17b1c944e05fd639ba8b5e60ccc` |
| `target/proof/local-qwen05-capsule-queue-proof-2026-07-03/long-chat-capsule-queue.exit` | 2 bytes | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |
| `target/proof/local-qwen05-capsule-queue-proof-2026-07-03/server.log` | 13 lines | `c208831734e4990df06ea98201ebbf72aec2c8e1d12dd04e574c2c4cdf081584` |
| `target/proof/local-qwen05-capsule-queue-proof-2026-07-03/server.stdout` | 0 bytes | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

## Queue Probe

```text
long_chat_queue_probe_holder_prompt_cache_key=ferrite:qwen05:q4:capsule-queue:a:256:2026-07-03
long_chat_queue_probe_contender_prompt_cache_key=ferrite:qwen05:q4:capsule-queue:b:256:2026-07-03
long_chat_queue_probe_holder_started_streaming=true
long_chat_queue_probe_holder_completed=true
long_chat_queue_probe_contender_status=200
long_chat_queue_probe_contender_completed=true
long_chat_queue_probe_contender_generated_event=true
long_chat_queue_probe_contender_started_after_holder=true
long_chat_queue_probe_max_tokens=64
```

## Capsule Identity Results

The full assistant-context hash differs from the generated-context hash on
follow-up turns because the assistant context includes the state capsule wrapper.
The carried generated-context hash still matches the previous generated-response
hash.

| Lane | Turn | Generated context hash | Generated response hash | TTFT ms | Decode tok/s | RSS before | RSS idle |
| --- | ---: | --- | --- | ---: | ---: | ---: | ---: |
| A | 1 | n/a | `fnv64:0521e53e3d44ce4e` | 73 | 20.915412 | 451510272 | 414089216 |
| A | 2 | `fnv64:0521e53e3d44ce4e` | `fnv64:225084dc02b200c7` | 15024 | 16.932003 | 414089216 | 422232064 |
| A | 3 | `fnv64:225084dc02b200c7` | `fnv64:f4e3589b9b2aae8b` | 12801 | 16.702832 | 422232064 | 428916736 |
| A | 4 | `fnv64:f4e3589b9b2aae8b` | `fnv64:cf26fafe27cd1834` | 12740 | 17.023780 | 428916736 | 448135168 |
| B | 1 | n/a | `fnv64:0521e53e3d44ce4e` | 79 | 22.084828 | 448135168 | 448610304 |
| B | 2 | `fnv64:0521e53e3d44ce4e` | `fnv64:225084dc02b200c7` | 14299 | 16.694559 | 448610304 | 426409984 |
| B | 3 | `fnv64:225084dc02b200c7` | `fnv64:f4e3589b9b2aae8b` | 12563 | 17.152616 | 426409984 | 436420608 |
| B | 4 | `fnv64:f4e3589b9b2aae8b` | `fnv64:cf26fafe27cd1834` | 12755 | 16.950184 | 436420608 | 426459136 |

Every scenario reported 256 content chunks, 256 token-id chunks, valid usage
accounting, and `long_chat_result_hit_token_limit=true`.

## Integrated Summary

```text
long_chat_summary_planned_scenarios=8
long_chat_summary_completed_scenarios=8
long_chat_summary_generated_follow_up_turns=6
long_chat_summary_cached_generated_follow_up_turns=6
long_chat_summary_uncached_generated_follow_up_turns=0
long_chat_summary_all_generated_follow_up_turns_cached=true
long_chat_summary_all_follow_up_turns_use_generated_context=true
long_chat_summary_generated_context_identity_links=6
long_chat_summary_matching_generated_context_identity_links=6
long_chat_summary_all_generated_context_identity_links_present=true
long_chat_summary_all_generated_context_identities_match_previous_response=true
long_chat_summary_all_timing_present=true
long_chat_summary_all_streaming_token_id_summaries_present=true
long_chat_summary_all_streaming_content_chunks_have_token_ids=true
long_chat_summary_all_rss_present=true
long_chat_summary_error_probe_completed=true
long_chat_summary_disconnect_probe_completed=true
long_chat_summary_queue_probe_completed=true
long_chat_summary_queue_probe_contender_started_after_holder=true
long_chat_summary_run_complete=true
```

## Interpretation

The capsule-aware identity fix is proven against a real OpenAI-compatible
streaming path for the local Qwen2.5-0.5B Q4_K_M model. State capsules can wrap
assistant context without invalidating continuity proof, as long as the carried
generated response is still preserved and reported through
`long_chat_result_generated_context_hash`.

This proof is local and uses the queue probe's warmed-key behavior. It should
not be used as cold-key cache-isolation evidence. The Qwen2.5-1.5B Q8_0
x86_64 mixed-key proof remains the isolation reference.

## Remaining Work

- Repeat this capsule queue proof on staging with Qwen2.5-1.5B Q8_0 when the
  Kubernetes API is reachable.
- Add a pure cold-key queue variant if queue behavior must also prove namespace
  isolation.
