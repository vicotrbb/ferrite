# Theory: Structured Continuity Anchors

Date: 2026-07-03 UTC, 2026-07-02 local time

Status: New theory with first positive and negative probes

## Hypothesis

Generated-context windowing should not be treated only as a token-budget
problem. For long local OpenAI-compatible chat, the shape of retained state may
matter as much as the number of retained generated tokens.

A compact, structured continuity anchor can preserve critical state with fewer
tokens than raw assistant prose. A long arbitrary marker, or a fact buried in
natural-language completion text, may be too brittle when the window is small.

## First Evidence

The 256-token x86_64 Qwen2.5-1.5B Q8_0 continuity probe tested the current
32-token and 64-token generated-context windows with the new
`--require-generated-response-contains` assertion.

Negative result:

- Anchor: `FERRITE-CONTINUITY-7291`
- Windows: 32 and 64 generated-token chunks
- Result: both failed at turn 2 with the generated response missing the required
  full marker.

Positive result:

- Anchor: `7291`
- Windows: 32 and 64 generated-token chunks
- Result: both completed four streaming turns, RSS sampling, error probe,
  disconnect/reconnect probe, usage accounting, finish reason checks, token
  limit status, and streaming token ID checks.

Benchmark evidence:

- `documentation/benchmarks/2026-07-03-openai-long-chat-x86-qwen-1-5b-q8-continuity-window-256.md`

## Mechanism

The current generated-context gate carries prior assistant output into the next
chat turn. Windowing reduces that generated context, which cuts prompt tokens
and TTFT but can also erase important facts.

Structured continuity anchors would make important retained state compact and
easy for the next prompt to recover. Candidate formats:

- a short numeric or base32 anchor, such as `state_anchor=7291`;
- a small JSON footer, such as `{"state_anchor":"7291","topic":"cpu-risk"}`;
- a server-side state capsule appended to the next prompt outside assistant
  prose;
- a rolling summary capsule constrained to a small token budget;
- a short checksum-like continuity key generated from the conversation state.

This should be tested as a proof harness first, not shipped as invisible
serving behavior.

## Experiment Matrix

Test anchor shape while holding model, prompt, and generated-context window
constant:

| Variant | Example | Expected signal |
| --- | --- | --- |
| Long label | `FERRITE-CONTINUITY-7291` | Fails if arbitrary text is brittle |
| Short numeric | `7291` | Passes if compact anchors survive |
| Key/value | `state_anchor=7291` | Tests structured natural text |
| JSON footer | `{"state_anchor":"7291"}` | Tests machine-shaped state |
| State capsule | separate model-facing state block | Tests non-prose continuity |
| Summary capsule | one-sentence state summary plus anchor | Tests semantic continuity |

For each variant, run 32, 64, and 128 generated-token windows at 256 and 1024
completion budgets. Require:

- four streaming turns;
- generated context on follow-up turns;
- `finish_reason` and token-limit status;
- usage accounting;
- streaming token IDs;
- RSS before/after samples;
- unauthorized reconnect coverage;
- disconnect/reconnect coverage;
- generated response contains the required anchor;
- a semantic continuity check for facts that are not exact substrings.

## Design Constraints

- Do not let proof-only state capsules leak into default serving semantics.
- Do not claim general conversation memory from substring recall.
- Do not hide truncation from clients if a future HTTP policy changes what
  history is sent to the model.
- Keep the Rust implementation split by concern: gate config, request building,
  assertion/reporting, and benchmark docs should stay separate.
- Preserve OpenAI-compatible response shapes even when Ferrite adds optional
  local extensions for proof or diagnostics.

## Current Read

The first pass says 32-token and 64-token windows remain viable, but the
continuity contract needs a better state representation than raw generated text
alone. The next highest-value implementation theory is a proof-only state
capsule mode in the long-chat gate, followed by a 1024-token run with both
substring and semantic checks.

## State Capsule Probe

Commit `f202a84` added a proof-only
`--generated-context-state-capsule TEXT` mode to the long-chat gate. The first
real-model run is documented in
`documentation/benchmarks/2026-07-03-openai-long-chat-x86-qwen-1-5b-q8-state-capsule-256.md`.

The result was mixed:

| Variant | Window 32 | Window 64 | Read |
| --- | --- | --- | --- |
| JSON state capsule with `7291` | passed 4 turns | failed at turn 2 | capsule placement is not robust |

The 32-token window passed four 256-token streaming turns with generated
follow-up context, RSS sampling, error probe, disconnect/reconnect probe,
streaming token IDs, usage accounting, finish reason checks, and token-limit
status. The 64-token window completed probes and turn 1, then failed the
required generated-response substring check on turn 2.

This is a useful falsification slice. It suggests that preserving more generated
assistant prose can compete with the structured capsule. The next theory should
test capsule placement and authority, not only window size.

## Follow-Up Placement Probe

Commit `211360a` tested the next placement variant on the same Qwen 1.5B Q8
x86_64 path. The benchmark note is
`documentation/benchmarks/2026-07-03-openai-long-chat-x86-qwen-1-5b-q8-state-capsule-follow-up-64.md`.

This run kept the 64-token generated-context window and the JSON capsule, but
placed the capsule in the follow-up user message:

```text
long_chat_generated_context_state_capsule_placement=follow-up
long_chat_generated_context_max_tokens=64
long_chat_required_generated_response_substrings=7291
```

Result:

| Variant | Window 64 | Read |
| --- | --- | --- |
| JSON state capsule in assistant context | failed at turn 2 | capsule competed with retained assistant prose |
| JSON state capsule in follow-up message | completed 4 turns | user-message placement preserved the anchor |

The follow-up-placement command exited `0` and completed error and
disconnect/reconnect probes, RSS sampling, four 256-token streaming turns,
usage accounting, `finish_reason=length`, token-limit checks, and streaming
token IDs. It also preserved the required `7291` substring through turns 2-4.

The important caveat is that `long_chat_summary_run_complete=false` because the
64-token generated-context window intentionally truncates previous generated
responses. That makes full generated-context identity matching fail even when
the continuity-anchor assertion passes.

This strengthens the authority-placement part of the theory: putting compact
state in the follow-up user message is more robust than injecting it into the
assistant-context block next to uncontrolled retained prose. It also costs
tokens and TTFT. Generated follow-up turns used 162 prompt tokens and averaged
`38759.67` ms TTFT.

## Short Follow-Up Capsule Probe

Commit `ed47c8f` tested the next capsule-shape variant on the same Qwen 1.5B
Q8 x86_64 path. The benchmark note is
`documentation/benchmarks/2026-07-03-openai-long-chat-x86-qwen-1-5b-q8-short-state-capsule-follow-up-64.md`.

This run kept the follow-up user-message placement, 64-token generated-context
window, and required anchor, but replaced the JSON capsule with:

```text
state_anchor=7291
```

Result:

| Variant | Window 64 | Generated prompt avg | TTFT avg | Read |
| --- | --- | ---: | ---: | --- |
| JSON capsule in follow-up message | completed 4 turns | 162.00 | 38759.67 ms | anchor preserved |
| Short capsule in follow-up message | completed 4 turns | 151.00 | 36127.00 ms | anchor preserved with lower prompt cost |

The shorter capsule reduced generated follow-up prompt cost by 11 tokens on
average and TTFT by `2632.67` ms while preserving the exact `7291` anchor
through turns 2-4.

This strengthens the compact-anchor part of the theory. The result is still
not a full long-chat identity-gate pass: `long_chat_summary_run_complete=false`
because the 64-token generated-context window intentionally truncates prior
output, so generated-context identity does not match the previous full
response.

## Capsule-Only Probe

Commit `d9161d3` added and tested
`--generated-context-state-capsule-placement assistant-context-only`. The
benchmark note is
`documentation/benchmarks/2026-07-03-openai-long-chat-x86-qwen-1-5b-q8-capsule-only-64.md`.

This run kept the short `state_anchor=7291` capsule and the same 64-token
window configuration, but omitted retained generated assistant prose from
follow-up turns. Generated follow-up assistant context became only:

```text
Ferrite state capsule:
state_anchor=7291
```

Result:

| Variant | Window 64 | Generated prompt avg | TTFT avg | Response identity |
| --- | --- | ---: | ---: | --- |
| JSON capsule in follow-up message | completed 4 turns | 162.00 | 38759.67 ms | changing |
| Short capsule in follow-up message | completed 4 turns | 151.00 | 36127.00 ms | changing |
| Short capsule as assistant context only | completed 4 turns | 80.00 | 18775.33 ms | fixed point on turns 2-4 |

Capsule-only placement preserved the exact `7291` anchor while dropping
generated follow-up prompt cost by 71 tokens and TTFT by `17351.67` ms versus
the short follow-up capsule run. It also produced the same generated response
hash on turns 2-4: `fnv64:201ea36ecbb7d57c`.

This is the strongest signal so far that, for this prompt, retained generated
assistant prose is not needed for the exact-anchor continuity contract and may
mostly add prompt cost and response drift. It still does not prove semantic
continuity or production serving policy.

The integrated long-chat summary remains intentionally incomplete:
`long_chat_summary_run_complete=false`. Capsule-only placement replaces the
previous full generated response, so it cannot satisfy full generated-context
identity matching by design.

## Semantic Capsule-Only Probe

Commit `414f0f0` tested whether capsule-only placement can preserve a named
fact rather than only an arbitrary numeric anchor. The benchmark note is
`documentation/benchmarks/2026-07-03-openai-long-chat-x86-qwen-1-5b-q8-capsule-only-semantic-64.md`.

The state capsule was:

```text
risk=thermal_throttling mitigation_code=reduce_batch_size owner=runtime_scheduler
```

The generated-response assertion required `reduce_batch_size`. Result:

| Variant | Window 64 | Generated prompt avg | TTFT avg | Response identity |
| --- | --- | ---: | ---: | --- |
| Short capsule as assistant context only | completed 4 turns | 80.00 | 18775.33 ms | fixed point on turns 2-4 |
| Semantic capsule as assistant context only | completed 4 turns | 74.00 | 17377.33 ms | fixed point on turns 2-4 |

The semantic capsule preserved the mitigation fact through turns 2-4 and
produced the same generated response hash on those turns:
`fnv64:7477d5f93ba8199e`.

This strengthens the theory beyond exact numeric-marker repetition. It still
uses a substring assertion, so it is not a general semantic-evaluation system.
It does show that capsule-only state can carry a compact named field through
the current OpenAI-compatible long-chat gate with lower prompt cost than the
numeric-anchor capsule-only run.

## Semantic Capsule-Only Prefix-Cache Probe

The next run repeated the semantic capsule-only lane with
`--experimental-prefix-cache`, `--prompt-cache-trace`, and
`--require-cached-follow-ups`. The benchmark note is
`documentation/benchmarks/2026-07-03-openai-long-chat-x86-qwen-1-5b-q8-capsule-only-semantic-cache-64.md`.

Result:

| Variant | Window 64 | Generated prompt avg | Cached prompt avg | TTFT avg | Cache read |
| --- | --- | ---: | ---: | ---: | --- |
| Semantic capsule only, no cache | completed 4 turns | 74.00 | 0.00 | 17377.33 ms | no cache key |
| Semantic capsule only, prefix cache | completed 4 turns | 75.00 | 58.00 | 4170.67 ms | all 3 follow-ups cached |

Turn 2 was a shared-prefix hit with `24` cached prompt tokens and `12295` ms
TTFT. Turns 3 and 4 were exact hits with `75` cached prompt tokens and TTFT of
`123` ms and `94` ms. The gate exited `0` and reported:

```text
long_chat_summary_cached_generated_follow_up_turns=3
long_chat_summary_uncached_generated_follow_up_turns=0
long_chat_summary_all_generated_follow_up_turns_cached=true
```

This is the strongest optimization signal in this theory so far. Capsule-only
state did not merely reduce prompt size; it also created a stable prompt shape
that Ferrite's OpenAI-compatible prefix cache could reuse almost completely on
later turns. Decode throughput stayed CPU-bound near 4 tokens per second, so
the win is specifically prefill and TTFT, not token generation speed.

The same identity caveat remains:
`long_chat_summary_run_complete=false`. Capsule-only placement replaces the
previous generated response, so it cannot satisfy full generated-context
identity matching by design.

## Next Steps

1. Test capsule placement in the follow-up user message instead of assistant
   context. Done for the 64-token Qwen 1.5B Q8 256-budget lane; it preserved
   the anchor but did not satisfy full generated-context identity because the
   window intentionally truncates prior output.
2. Test a shorter `state_anchor=7291` capsule against the JSON capsule. Done
   for the same 64-token Qwen 1.5B Q8 256-budget lane; it preserved the anchor
   with lower prompt cost and lower TTFT than the JSON capsule.
3. Test a capsule-only generated follow-up mode that omits retained generated
   prose. Done for the same lane; it preserved the anchor, reduced prompt cost
   materially, and produced a fixed response hash across turns 2-4.
4. Add a semantic recall probe that checks a short generated answer for a known
   fact without requiring the exact full marker. Done for one
   `mitigation_code=reduce_batch_size` capsule-only lane; broader semantic
   continuity remains unproven.
5. Test whether capsule-only semantic fixed points become cacheable with the
   OpenAI-compatible prefix cache. Done for the same lane; all three generated
   follow-up turns were cached, and exact hits on turns 3-4 reduced TTFT to
   sub-125 ms.
6. Repeat the cache proof at 512-token and 1024-token budgets before claiming
   long-output stability.
7. Only after those pass, draft an HTTP serving policy that makes truncation and
   state retention explicit to clients.
