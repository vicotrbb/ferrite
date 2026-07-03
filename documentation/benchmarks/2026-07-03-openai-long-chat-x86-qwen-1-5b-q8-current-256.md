# Benchmark: x86_64 Qwen 1.5B Q8 Current 256 Long-Chat Gate

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Run a current-commit lifecycle/cache proof for the required Tier 1
`Qwen2.5-1.5B-Instruct-Q8_0` OpenAI-compatible HTTP artifact at the
256-token long-chat budget.

This is the smallest larger-model slice after the Qwen 0.5B paired ladder:

- one required larger Tier 1 model;
- one required completion-token length;
- four repeated streaming chat turns;
- generated assistant context carried into every follow-up turn;
- prompt-cache trace and required cached follow-ups;
- token IDs, timing, latency-per-token summary, RSS, error probe, and
  disconnect/reconnect probe.

This is partial evidence. It does not close the full Tier 1 long-chat gate.

## Environment

- Ferrite commit: `087b8de`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-qwen15-q8-256-current`
- Node: `homelab-01`
- Pod IP: `10.42.248.196`
- Container image: `rust:1.96-bookworm`
- Architecture: `x86_64`
- CPU feature evidence: `/proc/cpuinfo` included `avx2`
- CPU request: `500m`
- CPU limit: `2`
- Memory request: `1Gi`
- Memory limit: `8Gi` (`memory.max=8589934592`)
- Ephemeral-storage request: `8Gi`
- Ephemeral-storage limit: `12Gi`
- Workspace size after source copy, model copy, release build, and proof:
  `2.0G`
- Pod cgroup memory current after proof: `2838179840` bytes
- Pod cgroup memory peak after build and proof: `4675682304` bytes
- Raw proof artifacts copied locally:
  `target/proof/x86-qwen15-q8-256-current-2026-07-03/`

The staging API had a brief etcd readiness interruption before this run. It
recovered before pod creation. The pod was deleted after artifact collection,
and a final
`kubectl --context staging get pod ferrite-avx2-qwen15-q8-256-current --ignore-not-found`
returned no pod output. Both staging nodes were Ready after cleanup.

## Model

- Model: `Qwen2.5-1.5B-Instruct-Q8_0`
- Pod path: `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
- SHA256:
  `d7efb072e7724d25048a4fda0a3e10b04bdef5d06b1403a1c93bd9f1240a63c8`

## Binaries

The binaries were built inside the amd64 pod. `file` reported both as
`ELF 64-bit LSB pie executable, x86-64`.

- `target/release/ferrite-server` SHA256:
  `c6e52e0858d8676d54636c0ef004e3b17b6f9b2f03890a86fc5ca97d462b3bac`
- `target/release/ferrite-openai-long-chat-gate` SHA256:
  `74e495ab2cf63aa2d18899498ead0ce53c677d3b92618109eba28e79e9a1386c`

Build result:

```text
Finished `release` profile [optimized] target(s) in 44.01s
```

## Server

```sh
./target/release/ferrite-server \
  --bind 127.0.0.1:18195 \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --model-id Qwen2.5-1.5B-Instruct-Q8_0 \
  --api-key local-secret \
  --default-max-tokens 256 \
  --hard-max-tokens 512 \
  --inference-wait-ms 120000 \
  --experimental-prefix-cache
```

Readiness:

```text
GET /health -> {"status":"ok","ready":true,"model":"Qwen2.5-1.5B-Instruct-Q8_0"}
```

Server PID for RSS sampling: `1643`.

## Gate

```sh
./target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --addr 127.0.0.1:18195 \
  --api-key local-secret \
  --models Qwen2.5-1.5B-Instruct-Q8_0 \
  --token-lengths 256 \
  --turns 4 \
  --probe-max-tokens 256 \
  --rss-pid 1643 \
  --prompt-cache-key ferrite:qwen15:q8:256:current-2026-07-03 \
  --prompt-cache-trace \
  --require-cached-follow-ups \
  --expect-finish-reason length \
  --proof-log target/proof/x86-qwen15-q8-256-current-2026-07-03/x86-qwen15-q8-256-current.log \
  --proof-exit-code target/proof/x86-qwen15-q8-256-current-2026-07-03/x86-qwen15-q8-256-current.exit
```

The command exited `0` and wrote 214 proof-log lines.

## Probe Results

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_error_probe_reconnect_generated_event=true
long_chat_error_probe_reconnect_started_new_generation=true
long_chat_error_probe_max_tokens=256
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
long_chat_disconnect_probe_reconnect_generated_event=true
long_chat_disconnect_probe_reconnect_started_new_generation=true
long_chat_disconnect_probe_max_tokens=256
```

## Scenario Results

| Turn | Context | Prompt | Cached | Lookup | Response hash | TTFT ms | Stream tok/s | Decode tok/s | RSS idle |
| ---: | --- | ---: | ---: | --- | --- | ---: | ---: | ---: | ---: |
| 1 | seed | 43 | 0 | `miss` | `fnv64:2b46ddfa50fdcf15` | 10003 | 3.517333 | 4.059434 | 1946402816 |
| 2 | generated | 287 | 12 | `shared_prefix_hit` | `fnv64:9bbbc743c206d034` | 67923 | 1.857013 | 3.632738 | 1978384384 |
| 3 | generated | 287 | 34 | `shared_prefix_hit` | `fnv64:5137dd192dda0ce9` | 62654 | 1.928671 | 3.626173 | 1995554816 |
| 4 | generated | 282 | 34 | `shared_prefix_hit` | `fnv64:39eb905e38437a75` | 61136 | 1.950731 | 3.625616 | 2000130048 |

Every scenario reported:

```text
long_chat_result_finish_reason=length
long_chat_result_hit_token_limit=true
long_chat_result_streaming_content_chunks=256
long_chat_result_streaming_token_id_chunks=256
long_chat_result_streaming_token_ids=256
long_chat_result_streaming_all_content_chunks_have_token_ids=true
```

## Integrated Summary

```text
long_chat_summary_planned_scenarios=4
long_chat_summary_completed_scenarios=4
long_chat_summary_all_finish_reasons_present=true
long_chat_summary_all_usage_accounting_valid=true
long_chat_summary_all_token_limit_status_present=true
long_chat_summary_any_token_limit_hit=true
long_chat_summary_prompt_cache_key_present=true
long_chat_summary_cached_follow_ups_required=true
long_chat_summary_any_cached_prompt_tokens=true
long_chat_summary_generated_follow_up_turns=3
long_chat_summary_cached_generated_follow_up_turns=3
long_chat_summary_uncached_generated_follow_up_turns=0
long_chat_summary_all_generated_follow_up_turns_cached=true
long_chat_summary_generated_follow_up_context_required=true
long_chat_summary_all_follow_up_turns_use_generated_context=true
long_chat_summary_generated_context_identity_required=true
long_chat_summary_generated_context_identity_links=3
long_chat_summary_matching_generated_context_identity_links=3
long_chat_summary_all_generated_context_identity_links_present=true
long_chat_summary_all_generated_context_identities_match_previous_response=true
long_chat_summary_all_timing_present=true
long_chat_summary_streaming_token_ids_required=true
long_chat_summary_all_streaming_token_id_summaries_present=true
long_chat_summary_all_streaming_content_chunks_have_token_ids=true
long_chat_summary_rss_required=true
long_chat_summary_all_rss_present=true
long_chat_summary_error_probe_required=true
long_chat_summary_error_probe_completed=true
long_chat_summary_error_probe_reconnect_started_new_generation=true
long_chat_summary_disconnect_probe_required=true
long_chat_summary_disconnect_probe_completed=true
long_chat_summary_disconnect_probe_reconnect_started_new_generation=true
long_chat_summary_run_complete=true
```

## Server Lifecycle

The server emitted seven lifecycle lines:

```text
openai_stream_lifecycle request_id=stream-0 finish_reason=completed disconnect_point=none prompt_tokens_started=43 prompt_cancellation_polls=1247 generated_chunks=256 generated_token_ids=256 elapsed_ms=73735
openai_stream_lifecycle request_id=stream-1 finish_reason=cancelled disconnect_point=token_streaming prompt_tokens_started=0 prompt_cancellation_polls=0 generated_chunks=2 generated_token_ids=2 elapsed_ms=303
openai_stream_lifecycle request_id=stream-2 finish_reason=completed disconnect_point=none prompt_tokens_started=0 prompt_cancellation_polls=0 generated_chunks=256 generated_token_ids=256 elapsed_ms=62780
openai_stream_lifecycle request_id=stream-3 finish_reason=completed disconnect_point=none prompt_tokens_started=43 prompt_cancellation_polls=1247 generated_chunks=256 generated_token_ids=256 elapsed_ms=73066
openai_stream_lifecycle request_id=stream-4 finish_reason=completed disconnect_point=none prompt_tokens_started=275 prompt_cancellation_polls=7975 generated_chunks=256 generated_token_ids=256 elapsed_ms=138394
openai_stream_lifecycle request_id=stream-5 finish_reason=completed disconnect_point=none prompt_tokens_started=253 prompt_cancellation_polls=7337 generated_chunks=256 generated_token_ids=256 elapsed_ms=133252
openai_stream_lifecycle request_id=stream-6 finish_reason=completed disconnect_point=none prompt_tokens_started=248 prompt_cancellation_polls=7192 generated_chunks=256 generated_token_ids=256 elapsed_ms=131746
```

`stream-1` is the intentional disconnect-probe cancellation. The four scenario
streams completed without disconnect.

## Artifacts

| Artifact | Lines / bytes | SHA256 |
| --- | ---: | --- |
| `target/proof/x86-qwen15-q8-256-current-2026-07-03/x86-qwen15-q8-256-current.log` | 214 lines | `8b70cbf611cdf749308a9acc265fc69452cf44e6451ae078b4e183b3bd35ad93` |
| `target/proof/x86-qwen15-q8-256-current-2026-07-03/x86-qwen15-q8-256-current.exit` | 2 bytes | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |
| `target/proof/x86-qwen15-q8-256-current-2026-07-03/server.log` | 7 lines | `140243ca902fba9fd07123d1d0ddcf5e429ee691d7b08de03835e3b52ca79f81` |
| `target/proof/x86-qwen15-q8-256-current-2026-07-03/cgroup-memory-after.txt` | 3 lines | `054b7fdb74f2d45a42830f1d6c672556fa497240c34f46de39be725e23543c32` |

`sha256sums.txt` in the same artifact directory records the copied proof-file
hashes and was generated after the listed artifacts were collected.

## Interpretation

This run proves the current lifecycle/cache server can complete the 256-token
larger-model Q8 long-chat gate on x86_64 with generated context, token IDs,
RSS, latency summaries, prompt-cache trace, and reconnect/error probes.

The cache signal differs from the Qwen 0.5B 1024 fixed-point result. Qwen 1.5B
Q8 did not converge to an exact prompt-cache hit in this 256-token lane. The
follow-up turns stayed at shallow `shared_prefix_hit` reuse:

- turn 2: `12 / 287` cached, TTFT `67923` ms;
- turn 3: `34 / 287` cached, TTFT `62654` ms;
- turn 4: `34 / 282` cached, TTFT `61136` ms.

Decode throughput stayed near `3.63` token events/sec across generated turns,
while TTFT moved with prompt/cache cost. That keeps prefix identity and context
stabilization as the next optimization target for this model.

## Limits

This run does not prove:

- Qwen 1.5B Q8 512-token or 1024-token current lifecycle/cache behavior;
- Qwen 1.5B Q6 current lifecycle/cache behavior;
- SmolLM2 1.7B 256/512/1024 generated-output behavior;
- explicit stop or natural EOS behavior for Qwen 1.5B Q8;
- a 6Gi memory fit for Qwen 1.5B Q8;
- high-concurrency behavior;
- long-running RSS stability beyond this bounded proof.
