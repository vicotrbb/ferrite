# Theory: Qwen 0.5B Generated-Context Cache Stability

Date: 2026-07-03 UTC

Status: Testing

## Hypothesis

Qwen2.5-0.5B long-chat latency is dominated by whether generated follow-up
turns get stable token-prefix reuse. When the prompt cache reuses most or all
of a lane's prompt, time to first token collapses from minutes to milliseconds.
When reuse falls to a few tokens, TTFT dominates the OpenAI-compatible streaming
experience even though decode throughput remains comparatively bounded.

## Evidence

The x86_64 full-matrix proof completed on 2026-07-03. Benchmark note:

`documentation/benchmarks/2026-07-03-openai-long-chat-x86-qwen-0-5b-full-matrix.md`

The run passed with:

```text
long_chat_summary_completed_scenarios=12
long_chat_summary_generated_follow_up_turns=9
long_chat_summary_cached_generated_follow_up_turns=9
long_chat_summary_uncached_generated_follow_up_turns=0
long_chat_summary_all_generated_follow_up_turns_cached=true
long_chat_summary_run_complete=true
```

The pass proves protocol compatibility for this model and gate shape, but the
latency rows show unstable cache depth across generated-context lanes:

| Turn | Tokens | Prompt | Cached | TTFT ms | Decode tok/s | Elapsed ms |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 2 | 1024 | 1054 | 525 | 202997 | 2.180737 | 674587 |
| 3 | 1024 | 1054 | 16 | 372787 | 2.283376 | 823267 |
| 4 | 1024 | 1054 | 1054 | 277 | 2.375039 | 433453 |
| 3 | 512 | 542 | 306 | 82497 | 2.674020 | 275986 |
| 4 | 512 | 542 | 20 | 176883 | 2.656111 | 371663 |

The 1024-token lane is the clearest signal. Prompt tokens stayed constant at
1054 across generated follow-ups, but cached prompt tokens varied from 16 to
1054. TTFT moved with cache depth, while decode throughput stayed in a much
narrower band.

## Interpretation

This does not look like a pure model-throughput problem. It looks like a cache
stability and prefix-identity problem surfaced by generated assistant context.
The server can complete the full OpenAI-compatible long-chat matrix with bounded
RSS, but user-visible latency is not predictable enough for long-chat UX until
we can explain and stabilize cache reuse.

The result also changes how we should test: each token-length lane has its own
conversation history. Row order alone can be misleading; turn 3 / 256 does not
inherit the 1024-token lane's full transcript.

## Next Experiments

1. Deterministic prompt identity trace: record per-turn prompt token hashes,
   longest shared-prefix length, selected cache entry, and cache namespace for
   each lane. Acceptance: the reported cached token count can be explained from
   the trace without reading generated text manually. Implementation slice:
   `documentation/dev-notes/2026-07-03-prompt-cache-trace-observability.md`.
2. Lane-isolated replay: rerun only the 1024-token lane for four turns with the
   same prompt-cache key shape. Acceptance: cache depth and TTFT either repeat
   the collapse/recovery pattern or prove the previous run was nondeterministic.
3. Short diagnostic gate: run 128-token completions with the same generated
   follow-up mechanics and the new cache trace. Acceptance: failures can be
   reproduced quickly before spending full-matrix time.
4. Prefix serialization audit: compare rendered prompt bytes and token IDs
   before cache lookup. Acceptance: no string-level assumption is used where a
   token-level prefix decision is required.
5. Cache eviction pressure probe: rerun with lower and higher cache-entry limits
   after exposing the limit in proof output. Acceptance: TTFT changes correlate
   with explicit eviction events, not unexplained cache misses.

## Falsification Criteria

This theory is weakened if trace output shows that low cached-token rows are
caused by intentional eviction or by genuinely different token prefixes. It is
falsified if repeated 1024-lane runs show high TTFT even with full prompt reuse,
or low TTFT with minimal prompt reuse, because that would point away from prefix
cache depth and toward another bottleneck.

## Engineering Boundary

Do not optimize this by adding opaque state or broad session mutation. The next
code change should be narrow instrumentation first, placed in focused Rust
modules around prompt/cache observability. The output must be useful to both the
Ferrite long-chat gate and external OpenAI-compatible benchmarking tools such as
`llama-benchy`.

## Implementation Update

The first instrumentation slice now exists as an opt-in trace. It does not
change default OpenAI response shape and it does not claim cache behavior is
optimized. The next proof step is a traced real-model rerun so each low-cache or
high-TTFT row has a token-hash and selected-entry explanation.

## Trace Diagnostic Result

The first local traced diagnostic completed for Qwen2.5-0.5B Q4_K_M at 128
generated tokens:

`documentation/benchmarks/2026-07-03-openai-long-chat-qwen-0-5b-trace-diagnostic-128.md`

The run reported `long_chat_summary_run_complete=true`, three cached generated
follow-up turns, and no uncached generated follow-up turns. Each generated row
reported `lookup=shared_prefix_hit`, and `cached_prompt_tokens` matched
`shared_prefix_tokens`.

This strengthens the cache-stability theory because low reuse is now directly
visible: generated follow-ups reused only 14 to 16 prompt tokens out of roughly
157 to 158 prompt tokens, and TTFT stayed around 6.3 to 6.4 seconds. The result
does not prove an optimization; it proves the next optimization target is
measurable.
