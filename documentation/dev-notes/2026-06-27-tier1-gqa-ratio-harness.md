# 2026-06-27 Tier 1 GQA Ratio Harness

## Slice

Tier 0 model plumbing is complete under ADR 0005, so the next smallest Tier 1
correctness slice is to harden grouped-query attention broadcasting before any
SIMD or larger-model performance work.

This slice extracts scalar causal attention into
`crates/ferrite-inference/src/scalar/attention.rs` and adds a focused unit test
for the Tier 1 GQA ratios listed in the operating gate:

- 1:1
- 3:1
- 4:1
- 6:1
- 7:1

The test uses two KV heads and verifies that each query head reads the expected
KV value slice for the configured `heads_per_kv` ratio.

## Validation

Test-first failure before implementation:

```text
cargo test -p ferrite-inference gqa_broadcasts_kv_heads_for_tier1_ratios -- --nocapture
```

The new test failed against the initial stub with output values `[0.0, 0.0]`
where `[10.0, 11.0]` was expected.

Passing checks after implementation:

```text
cargo test -p ferrite-inference gqa_broadcasts_kv_heads_for_tier1_ratios -- --nocapture
cargo test -p ferrite-inference --test scalar_reference
```

The targeted GQA ratio test passed, and all 16 scalar reference integration
tests passed.

## Remaining Work

This is a scalar correctness harness only. It does not prove SIMD correctness,
throughput, or real 0.5B-1.7B Tier 1 model behavior. Next Tier 1 slices should
compare optimized kernels against this scalar attention path and then capture
larger-model reference evidence.
