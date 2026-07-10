# Dev note: vectorized Q5_0 NEON block-dot decode

- Date: 2026-07-09
- Commit context: successor to 2d9f0fb (dirty tree during work)
- Theory under test: `documentation/theories/2026-07-09-decode-bottleneck-scalar-dequant.md`

## What changed

`crates/ferrite-inference/src/scalar/q5_0_neon.rs`: `neon_q5_0_block_dot`
previously decoded each group of four 5-bit weights with scalar bit
surgery into `[f32; 4]` stack arrays before a 4-lane FMA. It now decodes
the whole 32-value block in NEON registers:

- one `vld1q_u8` load of the 16 quant bytes; nibbles split with
  `vandq_u8`/`vshrq_n_u8`;
- high bits expanded with a `vtstq_u8` bit-test against a per-lane mask
  table (`HIGH_BIT_LANE_MASK`), OR-ed into the nibbles, then `- 16` as
  `int8x16_t`;
- signed bytes widened `s8 → s16 → s32 → f32` (exact conversions) via
  the shared `widen_s8_lanes` helper.

The FMA accumulation order (low quad then high quad per 4-lane step,
single accumulator register, per-block `vaddvq_f32 × scale`) is exactly
the order of the previous kernel, so the result is bit-identical: the
only float operations, FMA into the same lanes in the same sequence,
horizontal add, scale multiply, are unchanged.

## Validation

- `cargo test --workspace --release`: 62/62 suites ok (full log
  verified; two earlier apparent one-test failures were artifacts of
  interleaved shell pipelines during parallel clippy/test runs and did
  not reproduce across three dedicated reruns of the suspect suites).
- New unit test sweeps six `high_bits` patterns (all-zero, all-one,
  mixed, single-bit edges) against `decode_q5_0_row`.
- Bit-identity proof: 64-token greedy generation for the eval prompt
  produced byte-identical `generated_token_ids` and `next_token_id`
  (9646) under the old kernel (git-stashed build, benchmark-fingerprint
  30.8–34.6 ms/token) and the new kernel (24.2–25.8 ms/token in the
  same thermal window).
- Gates: `cargo fmt --all --check`, `cargo clippy --workspace
  --all-targets -- -D warnings`, `cargo check -p ferrite-inference
  --target x86_64-unknown-linux-gnu --tests` all pass. Clippy 1.96
  surfaced four pre-existing violations in untouched code (fixed in the
  companion hygiene commit); `kv_store/locus.rs` had pre-existing
  rustfmt drift, now formatted.

## Measured result (Apple M5 Pro, 15 cores, macOS 26.5.2)

Official eval (`scripts/eval.py`, Qwen2.5-0.5B-Instruct Q4_K_M-labelled
GGUF, 64 generate tokens / 64 benchmark runs):

| record | decode tok/s (precise) | p50 / p95 ms |
| --- | --- | --- |
| `2026-07-09-191148` (baseline, 2d9f0fb) | 31.99 | 30.9 / 33.7 |
| `2026-07-09-185239` (earlier baseline record) | 35.98 | 26.8 / 29.6 |
| `2026-07-09-192611` (this slice) | 42.70 | 23.4 / 25.5 |

`--profile-next-token` role deltas (single token, same host):
q_proj 3.45→1.92 ms, o_proj 3.56→1.90 ms, ffn_gate 9.06→4.93 ms,
ffn_up 7.31→4.62 ms, k_proj 0.50→0.28 ms (24-layer totals).

## Follow-ups

- The kernel is now FMA-chain-latency-bound (single accumulator per
  block preserved for bit-identity). Next slice: independent per-row
  accumulator chains via row pairing (keeps per-row accumulation order,
  so still bit-identical), measured separately.
- Q6_K/Q4_K NEON block dots still use scalar lane arrays (ffn_down).
- Run-to-run spread on this host is ~±8%; slice acceptance uses
  median-of-3 benchmark runs plus the official eval record.
