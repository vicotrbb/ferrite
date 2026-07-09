# Dev note: vectorized Q6_K / Q4_K NEON block-dot decode

- Date: 2026-07-09
- Follows: `2026-07-09-thread-pool-and-rayon-granularity.md`
- Theory: `documentation/theories/2026-07-09-decode-bottleneck-scalar-dequant.md`

## What changed

Applied the slice-A treatment (vector-register weight decode, FMA order
preserved for bit-identical sums) to the two remaining scalar-lane-array
kernels, which after slices A+B were the largest per-token cost
(ffn_down: Q6_K 2.77 ms + Q4_K 1.82 ms of a 14.1 ms profiled token):

- `q6_k_neon.rs`: per 16-value window, the four 6-bit groups are decoded
  with two 16-byte `vld1q_u8` low-bit loads + one high-bit load,
  vectorized mask/shift/or, `- 32` in `int8x16_t`, exact widening via the
  new shared `neon_util::widen_s8_lanes`; scales precomputed per window.
  The previous kernel's FMA sequence (q1..q4 per 4-lane step, single
  accumulator, `vmulq` by scale before each FMA) is replayed exactly.
- `q4_k_neon.rs`: per 32-byte chunk, both 16-byte halves are loaded once
  and nibble-split in registers; `vsub(vmul(quad, d), min)` shape and
  the low-then-high FMA order are unchanged.
- `neon_util.rs` (new): shared exact `s8 → f32` widening helper; the
  Q5_0 kernel now uses it too.

## Validation

- Kernel unit tests vs scalar decoders pass; full workspace suite
  62/62 ok (counted with stderr separated after two earlier
  miscounts caused by interleaved cargo streams).
- Bit-identity: 64-token greedy generation ids identical to the
  slice-A capture.
- Gates: fmt, clippy `-D warnings` (one `needless_range_loop` in the new
  Q6_K loop restructured with zip/enumerate), x86_64 cross-check.

## Measured result (Apple M5 Pro, 10-thread pool)

- Direct benchmark medians (32 runs each, 3 samples): 15.5 ms → 14.4 ms
  per token (~64.5 → ~69.5 tok/s).
- Interleaved A/B vs the slice-B head (3 rounds, alternating builds in
  the same thermal window): 17.12/17.81/18.16 ms (B) vs
  16.02/16.59/16.18 ms (C) — slice C faster in every round by 6–9%.
- Official eval `2026-07-09-195002` (tag
  `q6k-q4k-neon-vectorized-decode`): 61.29 tok/s precise, p50 16.3 ms —
  statistically flat vs slice B's 62.06 record because the eval windows
  differ thermally; the interleaved A/B above is the controlled
  comparison and is what this slice's acceptance rests on. Two earlier
  eval attempts for this slice were DISCARDED as contaminated: one ran
  concurrently with workspace clippy/test compilation, one ran 10 s
  after a full build+test cycle (thermal); the retained record ran after
  a 2-minute idle cooldown.

## Measurement protocol (reaffirmed)

1. Nothing else running (no builds, no tests, no subagents).
2. ≥2 min idle cooldown after any compile/test burst.
3. Median of ≥3 direct benchmark runs, plus the official eval record.
4. Interleaved A/B (stash ↔ tree) when ambient drift is suspected.

## Follow-ups

- All hot kernels now decode in vector registers; remaining known
  single-stream levers: FMA-chain ILP (row pairing), per-token
  allocation churn, RoPE sin/cos table, attention GQA restructure.
