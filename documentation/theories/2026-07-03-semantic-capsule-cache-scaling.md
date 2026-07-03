# Theory: Semantic Capsule Cache Scaling

Date: 2026-07-03 UTC

Status: Positive at 256, 512, and 1024 tokens

## Hypothesis

Capsule-only generated context can create a stable prompt fixed point for
OpenAI-compatible long-chat follow-up turns. If the prompt shape is stable,
Ferrite's experimental prefix cache should remove most repeated prefill cost
without changing the decode-bound portion of the response.

This is a serving-shape optimization theory, not a general memory theory.

## Evidence

The same Qwen2.5-1.5B-Instruct Q8_0 semantic capsule was tested at 256 and 512
completion-token budgets:

```text
risk=thermal_throttling mitigation_code=reduce_batch_size owner=runtime_scheduler
```

Both runs required the model response to include `reduce_batch_size`, required
cached generated follow-up turns, sampled RSS, checked streaming token IDs,
covered unauthorized reconnect behavior, and covered client
disconnect/reconnect behavior.

| Budget | Follow-up prompt avg | Cached prompt avg | TTFT avg | Turn 3 TTFT | Turn 4 TTFT | Gate exit |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 256 | 75.00 | 58.00 | 4170.67 ms | 123 ms | 94 ms | 0 |
| 512 | 75.00 | 58.00 | 4082.33 ms | 80 ms | 80 ms | 0 |
| 1024 | 75.00 | 58.00 | 4088.67 ms | 83 ms | 101 ms | 0 |

Both runs reported:

```text
long_chat_summary_cached_generated_follow_up_turns=3
long_chat_summary_uncached_generated_follow_up_turns=0
long_chat_summary_all_generated_follow_up_turns_cached=true
```

## Read

The cache behavior scaled from 256 to 512 to 1024 output tokens. Turn 2
remained a shared-prefix hit, while turns 3 and 4 became exact hits after the
fixed prompt shape was seeded.

Decode throughput did not improve. That is expected because the cache removes
repeated prefill work, not token sampling and decode work. The 512-token and
1024-token runs therefore spent most of their wall-clock time producing
completion tokens even after TTFT dropped to sub-125 ms.

## Design Implications

- Capsule-only generated context is a useful proof-harness shape for testing
  fixed prompt caching.
- The cache should be evaluated separately from decode throughput; mixing the
  two hides the prefill win.
- Exact-hit follow-up turns are the strongest signal. Shared-prefix hits on
  turn 2 are still useful, but the stable state begins after one generated
  follow-up has seeded the cache.
- Any future production policy must make state retention explicit to clients.
  Hidden capsule behavior is still not proven as a serving contract.

## Limits

This theory does not yet prove:

- multi-client cache eviction behavior;
- model-family generality;
- cache correctness under varied follow-up text;
- a production memory policy for hidden or explicit state capsules;
- full generated-context identity preservation.

The `long_chat_summary_run_complete=false` marker remains expected for these
proofs because capsule-only placement intentionally replaces prior generated
assistant prose with a state capsule.

## Next Experiment

The next experiment should stop repeating the same fixed single-client lane and
stress a different risk:

- mixed cache keys under concurrent or queued clients;
- varied follow-up text that preserves the capsule but changes user wording;
- cache eviction behavior after several distinct prompt-cache keys;
- a paired no-cache 1024 run if we need a direct long-output baseline.
