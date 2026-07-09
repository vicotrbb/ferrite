# Dev note: Q5_0 batched ILP and row granularity

- Date: 2026-07-09
- Scope: AArch64 NEON batched Q5_0 matvec
- Baseline artifact: `scripts/evals/2026-07-09-202640-qwen2.5-0.5b-instruct-q4_k_m.json`
- Accepted slice artifact: `scripts/evals/2026-07-09-204644-qwen2.5-0.5b-instruct-q4_k_m.json`
- Clean final artifact: `scripts/evals/2026-07-09-205246-qwen2.5-0.5b-instruct-q4_k_m.json`

## Hypothesis

The Q5_0 batched block-dot kernel decoded weights once but completed one
stream's dependent eight-FMA chain before starting the next stream. Interleaving
four independent stream accumulators should expose instruction-level
parallelism without changing any stream's arithmetic order. Batched 896-row
attention projections also inherited the single-stream minimum of 128 rows per
Rayon task, yielding only seven tasks for a ten-worker pool; 64 rows per task
should improve batch load balance while retaining ample work per task.

## What changed

`q5_0_neon.rs` now:

1. Processes complete groups of four batch streams with four named NEON
   accumulators. For each stream, the low-then-high FMA order for steps 0..4 is
   unchanged. Remaining streams use the original one-stream loop.
2. Uses a batch-only Rayon minimum of 64 rows per task. Single-stream dispatch
   keeps its measured 128-row minimum unchanged.
3. Includes a full eight-stream block test that compares every batched result's
   raw `f32` bits with the single-stream kernel.

The four named accumulators are intentional: they keep the independent chains
visible to the compiler and avoid a dynamically indexed SIMD accumulator array
that could spill to the stack.

## Canonical evaluation

```sh
scripts/eval.sh \
  --model target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --generate-tokens 64 \
  --benchmark-runs 64 \
  --batch-streams 4 \
  --batch-streams 8 \
  --skip-server \
  --tag q5-batch-ilp-granularity-preliminary
```

Apple M5 Pro, ten Ferrite threads:

| record | single-stream | batch 4 aggregate | batch 8 aggregate | batch-8 CPU mean | parity |
| --- | ---: | ---: | ---: | ---: | --- |
| `2026-07-09-202640` pre-slice | 44.37 tok/s | 72.77 tok/s | 87.70 tok/s | 543.8% | engine tests only |
| `2026-07-09-204014` four-way ILP | 49.01 tok/s | 93.11 tok/s | 99.42 tok/s | 526.8% | exact |
| `2026-07-09-204644` ILP + 64-row batch tasks | 49.49 tok/s | 93.00 tok/s | **101.65 tok/s** | 545.2% | exact |

The accepted batch-8 comparison improves 87.70 to 101.65 tok/s (+15.9%)
while observed CPU is effectively identical (543.8% vs 545.2%). The batch-8
step fell from 91.22 ms to 78.70 ms. The cumulative single-stream result is
49.49 tok/s versus the committed 31.99 tok/s starting baseline (+54.7%); batch
8 is +217.8% against that starting rate. Single-stream variation in the table
is ambient host load and is not attributed to this batch-only slice.

After all commits and gates, a clean-tree cooldown run recorded 65.47 tok/s
single-stream, 103.58 tok/s at batch 4, and 117.49 tok/s at batch 8. This is a
104.7% single-stream improvement over the 31.99 tok/s starting baseline, and
the 100 tok/s target is met at batch 4 as well as batch 8. The same run kept
exact stream-0 parity at batch 2/4/8 and completed the server phase at 51.13
streamed tok/s.

## Validation

- Eight-stream Q5_0 block outputs are bit-identical to single-stream outputs.
- The real-model eval reports stream-0 token parity at batch 4 and batch 8.
- `cargo clippy --workspace --all-targets -- -D warnings` passes.
- `cargo test --workspace --release` passes after the independently documented
  TCP coalescing test fix.
- `cargo check -p ferrite-inference --target x86_64-unknown-linux-gnu --tests`
  passes; the new code is AArch64-gated and does not alter x86 dispatch.
- `python3 -m unittest scripts/eval_test.py`, rustfmt, and `git diff --check`
  pass.
