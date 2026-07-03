# Long-Chat Per-Turn Follow-Ups

Date: 2026-07-03

## Purpose

The SmolLM2 EOS fixed-point cache theory needs a falsification lane where the
follow-up prompt can change by turn. The long-chat gate previously supported
only one repeated `--follow-up` value, which was enough for fixed-answer lanes
but weak for changing-answer experiments.

## Change

- Added `--follow-ups TEXT[,TEXT...]` to the long-chat gate.
- The list must contain one entry per configured turn.
- When configured, the runner uses the follow-up matching the scenario turn.
- Existing `--follow-up` behavior remains the default when `--follow-ups` is
  not provided.
- State-capsule follow-up decoration wraps the selected per-turn follow-up.

## Validation

```text
cargo test -p ferrite-server --test long_chat_gate follow_up -- --nocapture
```

Result: passed, 14 tests.

## Boundary

This is a harness feature only. It does not change OpenAI request schemas,
runtime inference behavior, tokenization, cache internals, or server endpoints.
It enables the next two-lane EOS theory experiment.
