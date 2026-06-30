# Attention Cached Value Finite Guardrail

## Context

Scalar causal attention already routes query/key score calculation through
finite-checked dot products and finite-checked softmax. The weighted value
combiner still trusted cached value vectors after checking only their length,
so a non-finite cached value could propagate into attention output.

## Change

`causal_attention` now rejects non-finite cached value entries before adding the
weighted value slice into the query-head output.

## Verification

Run the focused regression:

```sh
cargo test -p ferrite-inference scalar::attention::tests::attention_rejects_non_finite_cached_values -- --nocapture
```
