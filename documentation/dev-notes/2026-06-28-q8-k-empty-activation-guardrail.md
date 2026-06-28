# Q8_K Empty Activation Guardrail

Date: 2026-06-28

## Scope

This slice tightens the approved Path B Q8_K activation-dot contract by making
`BlockQ8K::quantize_blocks` reject an empty activation slice directly.

The matrix adapters already rejected zero columns before calling activation
quantization. This change keeps that caller guard, but also makes the lower
level Q8_K block API enforce its own non-empty block collection invariant.

## Red

The new guardrail test first failed because `quantize_blocks(&[])` returned an
empty block vector:

```sh
cargo test -p ferrite-inference q8_k_rejects_empty_activation_block_collection -- --nocapture
```

Failure:

```text
test scalar::q8_k::tests::q8_k_rejects_empty_activation_block_collection ... FAILED
Error: InferenceError { message: "empty activation blocks must fail" }
```

## Green

`BlockQ8K::quantize_blocks` now returns:

```text
Q8_K activation length must not be zero
```

for an empty activation slice.

Focused verification:

```sh
cargo test -p ferrite-inference q8_k_rejects_empty_activation_block_collection -- --nocapture
cargo test -p ferrite-inference q8_k -- --nocapture
cargo test -p ferrite-inference q4_k_q8_k -- --nocapture
cargo test -p ferrite-inference q6_k_q8_k -- --nocapture
```

Results:

```text
q8_k_rejects_empty_activation_block_collection: 1 passed
q8_k: 32 passed
q4_k_q8_k: 9 passed
q6_k_q8_k: 9 passed
```

## Interpretation

This does not change the Q4_K/Q6_K x Q8_K arithmetic contract or default
dispatch policy. It closes a small standalone API hole so the Path B activation
block layer cannot represent a zero-block matvec input as a successful
quantization result.
