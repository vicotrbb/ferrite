# Argmax finite boundary guardrail

## Context

Ferrite uses deterministic argmax token selection on scalar logits. The public
`argmax` helper previously rejected empty inputs but allowed NaN and infinity
values to participate in float comparisons.

## Change

`argmax` now rejects any non-finite input before selecting an index. This keeps
token selection deterministic and fails fast when upstream math produces invalid
logits.

## Verification

- Red: `cargo test -p ferrite-inference --test scalar_reference argmax_rejects_non_finite_values -- --nocapture`
- Green: `cargo test -p ferrite-inference --test scalar_reference argmax_rejects_non_finite_values -- --nocapture`

