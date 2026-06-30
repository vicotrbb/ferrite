# Scalar Weight Vector Finite Guardrail

## Context

Scalar model validation checked vector lengths for norm weights and optional
attention biases, but it did not reject non-finite vector values. Matrix
constructors now reject non-finite matrix data, so vector weights need the same
boundary invariant before a `ScalarLlamaModel` can be constructed.

## Change

`validate_weights` now validates finite values for required scalar vectors
(`output_norm`, layer attention norms, and layer FFN norms) and optional bias
vectors (`q_bias`, `k_bias`, and `v_bias`) after checking their expected
lengths.

The regression test covers `output_norm` with NaN, positive infinity, and
negative infinity.

## Verification

Run the focused regression:

```sh
cargo test -p ferrite-inference --test scalar_reference scalar_weights_reject_non_finite_output_norm_values -- --nocapture
```
