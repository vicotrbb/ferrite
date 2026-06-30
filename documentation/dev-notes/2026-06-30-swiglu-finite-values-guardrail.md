# SwiGLU Finite Values Guardrail

## Context

`swiglu` checked shape compatibility for FFN gate and up-projection vectors, but
it did not reject NaN or infinity before evaluating the SiLU gate. Non-finite
intermediate activations could then propagate into residual hidden states.

## Change

`swiglu` now rejects non-finite values in both gate and up vectors before
computing the activation product.

## Verification

Run the focused regression:

```sh
cargo test -p ferrite-inference scalar::math::tests::swiglu_rejects_non_finite_values -- --nocapture
```
