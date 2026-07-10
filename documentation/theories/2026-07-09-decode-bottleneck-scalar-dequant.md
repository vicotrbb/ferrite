# Theory: Tier-1 decode throughput is bounded by scalar dequantization inside the NEON matvec kernels, not by DRAM bandwidth

- Status: hypothesis under test
- Date: 2026-07-09
- Owner: performance iteration loop (Claude session)
- Baseline evidence: `scripts/evals/2026-07-09-191148-qwen2.5-0.5b-instruct-q4_k_m.json` (31.99 tok/s precise, commit 2d9f0fb) and `scripts/evals/2026-07-09-185239-qwen2.5-0.5b-instruct-q4_k_m.json` (35.98 tok/s, same tree)

## Hypothesis

Qwen2.5-0.5B-Instruct "Q4_K_M" decode on the Apple M5 Pro is limited by
instruction throughput of scalar weight-dequantization inside the Q5_0
(and to a lesser degree Q6_K/Q4_K) NEON matvec kernels, plus Amdahl
serialization of the sub-threshold attention projections, not by memory
bandwidth and not by the HTTP layer.

## Evidence

1. `--profile-next-token` role summary (commit 2d9f0fb, this host):
   Q5_0 roles (ffn_gate 9.06 ms, ffn_up 7.31 ms, q_proj 3.45 ms,
   o_proj 3.56 ms, k_proj 0.50 ms) plus Q6_K/Q4_K ffn_down 8.17 ms
   consume ~24 ms of a ~27 ms token. The Q8_0 output matvec streams
   144.6 MiB in 1.24 ms (~117 GB/s, near hardware bandwidth), while the
   Q5_0 roles stream at 4–9 GB/s. All kernels walk the same `Vec<u8>`
   weight layout, so the gap is kernel code, not data placement.
2. The GGUF is mislabeled by its filename: parsing the header shows
   attn q/k/o, ffn_gate/up, and token_embd are Q5_0; attn_v and the
   untied output.weight are Q8_0; ffn_down alternates Q6_K/Q4_K.
   The hot kernel is therefore `q5_0_neon.rs`, whose block dot performs
   per-element scalar bit surgery into `[f32; 4]` stack arrays
   (~16 scalar ops per 4 weights) before a single-accumulator FMA.
   `q8_0_neon.rs` by contrast does `vld1_s8` → `vmovl` widening →
   4-lane FMA entirely in registers, and is ~15–30× faster per byte.
3. Threading: Q5_0/Q8_0 row-parallel gates require rows ≥ 4096, so
   q/k/v/o projections (896/128 rows) run single-threaded; generation
   CPU mean is ~757–790% of 1500% available.
4. Server overhead is <1% per token: the CLI-vs-server tok/s gap is TTFT
   accounting in the client metric (`streaming_tokens_per_second`
   divides by elapsed incl. prefill).
5. Rejected-experiment registry (`tier1-gate-status.md:369`): naive
   row-level rayon for Q8_0/Q5_0 regressed previously; the Q8_K
   integer-activation route has fixed-prompt parity failures and must
   stay opt-in. Both constrain the solution space below.

## Expected measurement

Rewriting the Q5_0 NEON block dot to decode nibbles + high bits with
vector instructions (`vld1q_u8`, `vandq_u8`/`vshrq_n_u8`, `vtstq_u8`
high-bit expansion, `vmovl` widening, f32 FMA) while keeping f32
activation arithmetic and the existing per-block
`horizontal-add × scale` accumulation shape should cut Q5_0 role time
by ≥2× and raise end-to-end decode from ~32–36 tok/s to ≥50 tok/s on
this host. Follow-up slices (Q6_K/Q4_K decode vectorization, chunked
parallelism for the 896-row projections, hot-loop allocation and
finite-scan reduction) target ≥80 tok/s single-stream.

## Falsification experiment

`scripts/eval.py` (precise in-process decode tok/s, median of ≥3
benchmark-only runs `--benchmark-runs 64` on a quiet machine) before and
after each slice, plus `--profile-next-token` role deltas. The theory is
falsified for a slice if the median does not improve beyond the ±8%
run-to-run spread measured at baseline, or if any parity gate fails
(scalar-oracle tolerance tests, fixed six-prompt token-id expectations,
`cargo test --workspace`).

## Risks / why it may fail

- Float summation-order changes inside a block can flip a narrow argmax
  margin on fixed-prompt parity tests (observed before with Q8_K
  activation drift). Mitigation: keep the per-block 4-lane accumulate →
  horizontal add → scale shape identical to the current NEON kernel.
- If the true limiter is DRAM latency to unaligned `Vec<u8>` rows, the
  vectorized decode will not reach Q8_0-class throughput (this would
  show as a plateau well below ~50 GB/s effective on Q5_0 roles).
- Rayon fork-join overhead may dominate once per-row work shrinks;
  thresholds may need re-tuning with chunked granularity (measured
  separately, never bundled with the kernel slice).
