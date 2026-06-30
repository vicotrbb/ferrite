# Projection Bias Finite Result Guardrail

## Context

Scalar Q/K/V projection bias support adds optional finite bias vectors after
projection matvecs and before RoPE. The helper checked bias length, but a finite
projection value plus a finite bias value could still overflow to a non-finite
query, key, or value activation.

## Change

`add_optional_bias` now rejects non-finite projection values, non-finite bias
values, and finite inputs whose addition result would become non-finite. The
helper preflights every result before mutating the projection buffer.

## Verification

Run the focused regression:

```sh
cargo test -p ferrite-inference scalar::session::tests::add_optional_bias_rejects_non_finite_results -- --nocapture
```
