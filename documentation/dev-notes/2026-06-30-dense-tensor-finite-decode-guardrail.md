# Dense Tensor Finite Decode Guardrail

## Context

Dense GGUF tensor decoding converted F32, F16, and BF16 payloads into `f32`
values without rejecting NaN or infinity. Downstream matrix and scalar weight
validation catches many of those cases later, but the tensor decoder is the
first owned boundary that can reject malformed dense tensor payloads with a
precise tensor name and value index.

## Change

The dense tensor decoders now validate decoded F32, F16, and BF16 values before
returning them. Non-finite payloads fail with `tensor {name} value {index} must
be finite`.

The regression test covers NaN, positive infinity, and negative infinity for all
three dense encodings.

## Verification

Run the focused regression:

```sh
cargo test -p ferrite-inference scalar::tensor::tests::dense_tensor_decoders_reject_non_finite_values -- --nocapture
```
