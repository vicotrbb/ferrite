# Dev note: inference thread-pool sizing + rayon task granularity

- Date: 2026-07-09
- Follows: `2026-07-09-q5-0-neon-vectorized-decode.md`
- Theory: `documentation/theories/2026-07-09-decode-bottleneck-scalar-dequant.md` (Amdahl/threading half)

## Evidence that motivated the slice

`RAYON_NUM_THREADS` sweep after the Q5_0 kernel fix (benchmark-runs 32,
Apple M5 Pro = 5 "Super" + 10 "Performance" logical cores per
`hw.nperflevels`): 1T 52.6 ms, 2T 29.9, 4T 18.8, 6T 18.4, 8T 20.6,
9T 15.6, 10T 15.7, 11T 16.9, 12T 16.8, 13T 16.9, 14T 17.5,
15T (default) 22.6. The default pool (all logical cores) was the worst
configuration measured; scaling saturated at ~2.9× because per-matvec
fork-join overhead and slow-cluster stragglers set tail latency, and
the 896-row attention projections ran fully serial.

This revisits the rejected-experiment registry
(`tier1-gate-status.md:369`) deliberately: the old regressions were
*naive per-row* scheduling. This slice uses coarse `with_min_len`
chunking instead, and every number above was measured on this head.

## What changed

1. `ferrite-inference/src/threading.rs` (new): global rayon pool built
   at startup. Thread-count resolution: explicit `--threads` flag >
   `FERRITE_THREADS` > `RAYON_NUM_THREADS` > platform probe. The probe
   uses the largest homogeneous perflevel core count on macOS
   (`sysctl hw.nperflevels` / `hw.perflevelN.logicalcpu`, no unsafe, no
   new deps), `available_parallelism` elsewhere. On this host that
   resolves to 10. CLI and server both print `inference_threads=`.
2. Q5_0 row-parallel gate: rows ≥ 4096 && cols ≤ 1024 → rows ≥ 512 with
   `.with_min_len(128)` rows per task, so q_proj/o_proj (896×896)
   parallelize with ~12 µs minimum task size.
3. Q4_K/Q6_K ffn_down kernels: `.with_min_len(64)` (896 rows × 4864
   cols ≈ 24 µs minimum task size).
4. `--threads N` flag on `ferrite` and `ferrite-server`.

Per-row arithmetic and output ordering are untouched, so results stay
bit-identical (verified: 64-token generation ids equal to the slice-A
capture).

## Measured result

Interleaved A/B (3 rounds, same thermal window): HEAD 22.5/22.8/22.5 ms
vs this slice 15.7/15.3/15.5 ms per token. Attribution: granularity +
coverage ≈ 3 ms at 15 threads (22.6→19.7); pool sizing ≈ 4 ms more
(19.7→15.5). An earlier bimodal run (36 ms outliers) did not reproduce
under the interleaved protocol and is attributed to transient host
noise.

Official eval `2026-07-09-193755` (tag `thread-pool-and-granularity`):
62.06 tok/s precise decode, p50 16.1 ms (vs 42.70 tok/s for slice A,
31.99 baseline `2026-07-09-191148`). Server streamed tok/s 48.4 — the
client metric still divides by elapsed-including-TTFT; decode-only
matches the CLI number.

## Follow-ups

- Q6_K/Q4_K block dots still decode with scalar lane arrays.
- Q5_0 kernel remains FMA-chain-bound; row-pairing ILP is the next
  kernel slice.
- Thread-count probe should eventually be validated on x86 (homelab
  pod) — `available_parallelism` fallback is the current behavior
  there.
