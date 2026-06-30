# Quantized Tensor Scale Finite Decode Guardrail

## Context

Matrix constructors already reject non-finite scale fields for Q8_0, Q5_0,
Q4_K, and Q6_K storage. The generic tensor-to-f32 decode path still decoded
those formats directly, so malformed quantized tensors could produce
non-finite values before reaching matrix construction or scalar weight
validation.

## Change

The quantized tensor decoders now reject non-finite f16 scale fields for Q8_0,
Q5_0, Q4_K, and Q6_K before dequantizing values.

The regression test was driven format by format: Q8_0, Q5_0, Q4_K, then Q6_K.

## Verification

Run the focused regression:

```sh
cargo test -p ferrite-inference scalar::tensor::tests::quantized_tensor_decoders_reject_non_finite_scale_values -- --nocapture
```
