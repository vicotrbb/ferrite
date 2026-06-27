# 2026-06-27 Tier 1 RoPE Head-Dim Harness

## Slice

Tier 1 requires RoPE coverage for `head_dim=64` and `head_dim=128` before
optimized attention paths can be trusted. This slice extracts RoPE from the
generic scalar math module into `crates/ferrite-inference/src/scalar/rope.rs`
and adds a focused unit test for both Tier 1 head dimensions.

The test verifies:

- the first even/odd pair rotates with the unscaled position angle;
- the last even/odd pair rotates with the dimension-scaled position angle; and
- the rotated vector keeps the original head dimension.

## Validation

Test-first failure before implementation:

```text
cargo test -p ferrite-inference rope_rotates_full_tier1_head_dimensions -- --nocapture
```

The new test failed against the temporary pass-through implementation because
the first pair stayed at `1.0` instead of rotating to `cos(3)`.

Passing checks after implementation:

```text
cargo test -p ferrite-inference rope_rotates_full_tier1_head_dimensions -- --nocapture
cargo test -p ferrite-inference --test scalar_reference
```

The targeted RoPE test passed, and all 16 scalar reference integration tests
passed.

## Remaining Work

This proves scalar RoPE behavior for the Tier 1 head dimensions only. SIMD or
platform-specific attention paths still need numerical comparison against this
scalar reference path before correctness claims are made.
