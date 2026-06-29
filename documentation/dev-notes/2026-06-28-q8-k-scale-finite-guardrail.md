# 2026-06-28 Q8_K Scale Finite Guardrail

## Slice

This slice hardens the approved Path B Q8_K activation-dot contract by making
activation-scale computation fail closed when the derived Q8_K scale is not
finite.

`BlockQ8K::quantize` already rejected non-finite activation values. The missing
edge case was a finite but extremely tiny dominant activation value that could
make `-127.0 / max` non-finite before quantization.

## Change

- Added a regression test for `f32::MIN_POSITIVE` as the dominant activation.
- Added explicit finite checks for both the inverse scale and stored scale.
- Kept the existing zero-block contract unchanged: all-zero activation blocks
  still produce a zero-scale, zero-quant block.

## Validation

Red test before the implementation:

```sh
cargo test -p ferrite-inference q8_k_rejects_non_finite_activation_scale -- --nocapture
```

Result before the implementation:

```text
test scalar::q8_k::tests::q8_k_rejects_non_finite_activation_scale ... FAILED
Error: InferenceError { message: "non-finite activation scale must fail" }
```

Focused green test after the implementation:

```sh
cargo test -p ferrite-inference q8_k_rejects_non_finite_activation_scale -- --nocapture
```

Result after the implementation:

```text
test scalar::q8_k::tests::q8_k_rejects_non_finite_activation_scale ... ok
```

## Boundary

This is a defensive guardrail. It does not change Path B dispatch policy,
promote Q8_K activation matvecs to default dispatch, or make new throughput
claims.
