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

## Next Steps

1. Add a proof-only gate mode that appends a compact state capsule to follow-up
   turns without changing the public server default.
2. Run 32, 64, and 128 generated-token windows with numeric, key/value, JSON,
   and summary-capsule anchors.
3. Add a semantic recall probe that checks a short generated answer for a known
   fact without requiring the exact full marker.
4. Only after those pass, draft an HTTP serving policy that makes truncation and
   state retention explicit to clients.
