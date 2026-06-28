# 2026-06-27 Tier 1 Q6_K Argmax Reduction Regression

## Scope

This slice tested whether replacing the aarch64 Q6_K argmax output path's
intermediate `Vec<(row, score)>` collection with a Rayon `try_reduce` would
improve Qwen2.5-1.5B throughput. The hypothesis was that avoiding the row-score
allocation for Q6_K output matrices would improve token-id-only generation.

The optimization is not retained.

## Baseline

Baseline was captured from the clean tree at `fcb35e8`.

Parity:

```text
generated_token_ids=198,9707,11
generated_match=true
```

Default Rayon-pool benchmark:

```text
benchmark_runs=5
benchmark_total_ns=1478415708
benchmark_avg_ns=295683141
        3.60 real         4.17 user         2.55 sys
          2097020928  maximum resident set size
          2268321472  peak memory footprint
```

Two-thread benchmark:

```text
benchmark_runs=5
benchmark_total_ns=1893387792
benchmark_avg_ns=378677558
        4.00 real         3.48 user         1.13 sys
          2092810240  maximum resident set size
          2268075584  peak memory footprint
```

## Experiment

The red test was a missing helper test for an aarch64 Q6_K argmax row-reduction
helper:

```text
error[E0432]: unresolved import `super::neon_q6_k_argmax_rows`
```

After implementation, the focused test and Q6_K filter passed:

```sh
cargo test -p ferrite-inference neon_q6_k_argmax_rows_matches_decoded_values -- --nocapture
cargo test -p ferrite-inference q6_k -- --nocapture
```

The release build also passed:

```sh
cargo build --release -p ferrite-cli
```

## Result

Parity after the experiment still matched:

```text
generated_token_ids=198,9707,11
generated_match=true
```

Default Rayon-pool benchmark after the experiment:

```text
benchmark_runs=5
benchmark_total_ns=1511808833
benchmark_avg_ns=302361766
        3.71 real         3.83 user         1.92 sys
          2082357248  maximum resident set size
          2268157568  peak memory footprint
```

Two-thread benchmark after the experiment:

```text
benchmark_runs=5
benchmark_total_ns=2968741542
benchmark_avg_ns=593748308
        5.77 real         3.65 user         1.72 sys
          2020605952  maximum resident set size
          2268124800  peak memory footprint
```

## Decision

The change regressed both measured variants:

| Run | Baseline benchmark_avg_ns | Experiment benchmark_avg_ns | Decision |
| --- | ---: | ---: | --- |
| Default Rayon pool | 295,683,141 | 302,361,766 | Rejected |
| `RAYON_NUM_THREADS=2` | 378,677,558 | 593,748,308 | Rejected |

The existing `Vec<(row, score)>` collection is not the bottleneck worth removing
with Rayon `try_reduce` for this model. Do not reapply this exact Q6_K argmax
row-reduction change without a new profile explaining why the scheduling and
reduction costs would differ.
