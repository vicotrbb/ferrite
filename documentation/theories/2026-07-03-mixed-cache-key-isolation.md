# Theory: Mixed Cache Key Isolation

Date: 2026-07-03 UTC

Status: Positive first proof at 256 tokens; queued local proof added

## Hypothesis

Ferrite's OpenAI-compatible prompt prefix cache should treat the prompt-cache
key as part of the cache namespace. Two clients or threads with identical prompt
tokens but different prompt-cache keys should not reuse each other's cache
entries.

This is a correctness and isolation theory first. It is not a decode-speed
optimization theory.

## Evidence

The first proof used Qwen2.5-1.5B-Instruct Q8_0 on the `staging` x86_64 pod
path with two prompt-cache keys:

```text
ferrite:qwen15:q8:mixed-cache:a:256:2026-07-03
ferrite:qwen15:q8:mixed-cache:b:256:2026-07-03
```

Benchmark evidence:

- `documentation/benchmarks/2026-07-03-openai-long-chat-x86-qwen-1-5b-q8-mixed-cache-keys-256.md`

Each lane had four 256-token streaming turns. Both lanes used the same semantic
state capsule and the same follow-up prompt.

| Lane | Turn 1 | Turn 2 | Turn 3 | Turn 4 |
| --- | --- | --- | --- | --- |
| A | miss | shared_prefix_hit | exact_hit | exact_hit |
| B | miss | shared_prefix_hit | exact_hit | exact_hit |

The critical signal is lane B turn 1: it was a miss even though the prompt token
hash matched lane A turn 1. That means the second key did not reuse the first
key's cache entry.

The summary markers also reported:

```text
long_chat_summary_generated_follow_up_turns=6
long_chat_summary_cached_generated_follow_up_turns=6
long_chat_summary_uncached_generated_follow_up_turns=0
long_chat_summary_all_generated_follow_up_turns_cached=true
```

## Read

The namespace design is behaving correctly for sequential mixed-key lanes. The
cache can warm and exact-hit independently per key while refusing cross-key
reuse for identical prompt-token hashes.

This is the right default for an OpenAI-compatible HTTP server because
`metadata.prompt_cache_key` is client-controlled and can represent a tenant,
thread, or conversation boundary.

## Queued-Client Follow-Up

A local Qwen2.5-0.5B Q4_K_M follow-up proved the queued-client harness against a
real OpenAI-compatible server:

- `documentation/benchmarks/2026-07-03-local-qwen-0-5b-queue-probe-256.md`
- `long_chat_queue_probe_holder_started_streaming=true`
- `long_chat_queue_probe_contender_status=200`
- `long_chat_queue_probe_contender_generated_event=true`
- `long_chat_queue_probe_contender_started_after_holder=true`
- `long_chat_summary_run_complete=true`

That proof confirms queued streaming clients can complete while using two
prompt-cache keys, and that the subsequent 8-scenario 256-token matrix keeps
generated-context identity, token IDs, RSS, reconnect, and timing summaries
valid.

It is not cold-lane isolation evidence: the queue probe warms both keys before
the normal scenario matrix, so lane B turn 1 reports `exact_hit` in the local
run. The x86_64 Qwen2.5-1.5B Q8_0 mixed-key proof above remains the stronger
cache-key namespace proof.

## Design Implications

- Mixed-key gate output must make lane identity explicit. Commit `1914057`
  adds `prompt_cache_key` to future `long_chat_result=...` lines.
- Summary logic must group generated-context identity by model, token length,
  and prompt-cache key. The mixed-key harness now does that.
- Throughput requests should prefer the scenario-specific key over the global
  single-key option. The mixed-key harness now does that.
- Queue probes should be documented separately from cold-lane namespace proofs
  because the holder and contender requests intentionally warm both keys.

## Limits

The sequential namespace proof plus local queue proof do not yet cover:

- true concurrent multi-permit inference;
- queued x86_64 Qwen2.5-1.5B Q8_0 behavior;
- cache eviction after many keys;
- varied follow-up text;
- 512-token or 1024-token mixed-key budgets;
- no-cache mixed-key baseline.

## Next Experiments

1. Run the same two-key proof at 512 and 1024 tokens only if 256-token behavior
   regresses or if we need long-output parity.
2. Repeat the queued-client probe on staging with Qwen2.5-1.5B Q8_0.
3. Add an eviction probe with more keys than the intended cache retention
   policy.
4. Add a varied-follow-up probe that preserves the capsule but changes user
   wording between turns.
