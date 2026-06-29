# 2026-06-28 Q8_0 x86 Warning Cleanup

## Slice

This slice removes cross-target warning noise from the Q8_0 parallel argmax
helper.

The helper is used by the aarch64 NEON output-argmax path and by its unit test.
On an x86_64 compile of the library target, the helper and its row-count
threshold were compiled but unused.

## Change

- Gated `PARALLEL_ARGMAX_MIN_ROWS` to `target_arch = "aarch64"`.
- Gated `parallel_argmax_q8_0_rows` and its Rayon import to aarch64 or tests.
- Kept the existing unit test compiled under `cfg(test)`.

## Validation

Failing gate before the cleanup:

```sh
RUSTFLAGS='-D warnings' cargo check -p ferrite-inference --target x86_64-unknown-linux-gnu --tests
```

Result before the cleanup:

```text
error: constant `PARALLEL_ARGMAX_MIN_ROWS` is never used
error: function `parallel_argmax_q8_0_rows` is never used
```

Focused green checks after the cleanup:

```sh
RUSTFLAGS='-D warnings' cargo check -p ferrite-inference --target x86_64-unknown-linux-gnu --tests
cargo test -p ferrite-inference parallel_argmax_q8_0_rows_matches_sequential_argmax -- --nocapture
```

Both commands passed.

## Boundary

This is a cross-target hygiene change only. It does not change Q8_0 argmax
selection on aarch64, AVX2 behavior on x86_64, or any benchmark claim.
