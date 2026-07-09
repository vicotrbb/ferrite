# Dev note: batched decode in the inference engine (ADR 0011 phase 2, engine half)

- Date: 2026-07-09
- Follows: concurrent serving phase 1; ADR 0011

## What changed

1. `Matrix::mul_vec_batch` / `Matrix::argmax_mul_vec_batch`: multiply
   several activation vectors in one weight pass. Batched NEON kernels
   for Q5_0 (decode-once per block, FMA into per-stream accumulators),
   Q8_0 (widen-once per block; plus a batched parallel argmax with
   explicit lowest-index tie-break), and Q6_K/Q4_K (row-cache-hot
   per-stream block dots). Non-aarch64 and other storage kinds fall back
   to per-vector matvecs. Per-stream block/FMA order matches the
   single-vector kernels exactly, so outputs are bit-identical per
   stream.
2. `scalar::accept_token_ids_batch(sessions, token_ids)`: advances N
   same-model sessions one token per weight pass (per-session norms,
   biases, RoPE, KV append, attention; batched q/k/v/o, ffn gate/up/down
   and logits argmax). Rejects empty batches, length mismatches, and
   cross-model batches.
3. CLI `--benchmark-batch-streams N` (with `--benchmark-runs`):
   prefills N sessions with the prompt and decodes them with the
   batched step, reporting `benchmark_batch_tokens_per_second` and
   stream 0's token ids for parity checking against a single-session
   run.

## Validation

- New `tests/batched_decode.rs`: batched step vs sequential sessions
  bit-for-bit over 8 steps on Q5_0 and Q8_0 fixtures (3 sessions with
  different prompt lengths); error-path tests. Full workspace: 63/63
  suites ok; fmt/clippy `-D warnings`/x86_64 cross-check green.
- Real model (Qwen2.5-0.5B eval GGUF): stream-0 `benchmark_token_ids`
  for batch 1, 2, 4, and 8 are byte-identical to the single-session
  benchmark sequence.

## Measured result (Apple M5 Pro, 10-thread pool, 32 decode steps)

| config | aggregate tok/s |
| --- | --- |
| single stream | ~69 (14.5 ms/token) |
| batch 2 | 92.5 |
| batch 4 | 115.9 |
| batch 8 | 137.7 |

Batch 8 ≈ 2× single-stream aggregate and ≈ 4.3× the session-start
baseline (31.99 tok/s). Scaling is sub-linear because per-session work
(attention, norms, allocation churn) and the per-stream FMA loops grow
with the batch; decode-once amortization currently covers Q5_0/Q8_0
only.

## Follow-ups

- Server half of ADR 0011 phase 2: scheduler-owned decode loop
  (continuous batching) so HTTP requests share weight passes; the
  engine API is ready for it.
- Decode-once batched inner loops for Q6_K/Q4_K.
- Per-stream scratch-buffer reuse in the batched step.
