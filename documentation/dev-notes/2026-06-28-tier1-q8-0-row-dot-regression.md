# Tier 1 Q8_0 Row-Dot Regression

Date: 2026-06-28

## Scope

This note records a rejected Q8_0 NEON kernel experiment for the local
Qwen2.5-1.5B-Instruct Q8_0 hot path.

The current-head profile in
`documentation/benchmarks/2026-06-28-tier1-qwen2-1-5b-q8-current-head-profile.md`
showed Q8_0 FFN and output roles dominating the benchmark token. The hypothesis
was that accumulating a whole Q8_0 row in NEON lanes and reducing once per row
would be faster than reducing once per Q8_0 block and adding scalar block
results.

## Experiment

The uncommitted experiment changed the aarch64 Q8_0 NEON path to:

- add a `neon_q8_0_row_dot` helper;
- multiply each block's quantized lanes by its scale before the FMA;
- horizontally reduce once per row; and
- route both `neon_q8_0_mul_vec` and `neon_q8_0_argmax_mul_vec` through that
  row-level helper.

No production code from this experiment was retained.

## TDD Evidence

Red test:

```sh
cargo test -p ferrite-inference neon_q8_0_row_dot_matches_decoded_values -- --nocapture
```

Initial result:

```text
error[E0432]: unresolved import `super::neon_q8_0_row_dot`
```

After implementation, focused checks passed:

```sh
cargo test -p ferrite-inference neon_q8_0 -- --nocapture
cargo test -p ferrite-inference q8_0_argmax_mul_vec_matches_full_matvec_argmax -- --nocapture
cargo clippy -p ferrite-inference --all-targets -- -D warnings
```

Observed result:

- Q8_0 NEON row/block tests passed.
- Q8_0 argmax matched full matvec argmax.
- `cargo clippy -p ferrite-inference --all-targets -- -D warnings` passed.

## Real Model Parity

The experiment preserved the fixed local Qwen2.5-1.5B Q8_0 prompt profile:

```sh
target/release/ferrite \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --prompt 'hello world' \
  --generate-tokens 6 \
  --expect-generated-token-ids 198,9707,11,1879,0,2585
```

Observed result:

```text
generated_token_ids=198,9707,11,1879,0,2585
generated_match=true
```

## Benchmark Regression

Baseline current-head profile before the experiment:

```text
benchmark_avg_ns=155274902
profile_benchmark_token_total_ns=155224492
```

Experiment command:

```sh
/usr/bin/time -l target/release/ferrite \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --prompt 'hello world' \
  --benchmark-runs 3 \
  --profile-benchmark-token
```

Experiment result:

```text
benchmark_total_ns=1386688875
benchmark_avg_ns=462229625
profile_benchmark_token_total_ns=452921003
profile_benchmark_token_role=ffn_down:Q8_0:1536:8960:14622720:122578959
profile_benchmark_token_role=ffn_gate:Q8_0:8960:1536:14622720:109689499
profile_benchmark_token_role=ffn_up:Q8_0:8960:1536:14622720:109564249
profile_benchmark_token_role=output:Q8_0:151936:1536:247959552:67292042
3709255680  maximum resident set size
3822493952  peak memory footprint
```

The experiment regressed the measured token from about 6.44 tok/s to about
2.16 tok/s and greatly increased max RSS in the timed run.

## Result

The row-level Q8_0 NEON accumulation experiment was reverted before commit.
Do not retry this exact shape. A future Q8_0 optimization needs a different
hypothesis, such as reducing scale-multiply overhead without extending live
lane pressure across the entire row, or focusing on output argmax scheduling
with direct evidence before and after the change.

## Revert Verification

After reverting the uncommitted experiment, the retained focused check passed:

```sh
cargo test -p ferrite-inference neon_q8_0 -- --nocapture
```

Observed result:

```text
test scalar::q8_0_neon::tests::neon_q8_0_block_dot_matches_decoded_values ... ok
```
