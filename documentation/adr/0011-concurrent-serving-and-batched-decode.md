# ADR 0011: Concurrent serving and batched decode

- Status: accepted for phase 1 (implemented, evidence in
  `documentation/benchmarks/2026-07-09-concurrent-serving-phase1.md`);
  phase 2 remains proposed
- Date: 2026-07-09

## Context

The server serializes all inference behind a single semaphore permit
(`state.rs`, `INFERENCE_PERMITS = 1`) and wraps the engine in
`Arc<Mutex<InferenceEngine>>` even though every generation entry point
takes `&self` and sessions borrow the model immutably. One request owns
the whole machine for its duration; a second request waits (or 429s).
Decode is memory-bandwidth-bound: a single stream streams ~406 MB of
weights per token and cannot saturate the memory system alone, so
serving N streams per weight pass is the main aggregate-throughput
lever on CPU.

## Decision

Phase 1, concurrent sessions (this ADR's implementation scope):

1. Drop the engine `Mutex`; hold `Arc<InferenceEngine>`. The engine is
   immutable after load except the prefix cache, which keeps its own
   internal `Mutex`.
2. Make the permit count configurable: `--max-concurrent-inferences N`
   (default 1, preserving current behavior and backpressure semantics
   exactly; the flag is the opt-in).
3. Each request still runs its own session on the shared rayon pool;
   concurrent matvec fork-joins interleave via work stealing. This
   trades single-stream latency for aggregate throughput; both are
   measured with the existing throughput client (`--concurrency`).

Phase 2, batched decode (design, separate implementation slices):

1. `ferrite-inference` gains batched matvec kernels
   (`mul_vec_batched(weights, &[activation])`) for Q5_0/Q6_K/Q4_K/Q8_0/
   F32 that stream each weight row once and dot it against B activation
   vectors. Per-stream dot order stays identical to the single-vector
   kernels, so batched results are bit-identical per stream.
2. A `BatchedDecode` step API advances B sessions one token per weight
   pass (per layer: per-stream norms/QKV/RoPE/attention over each
   stream's own KV, batched FFN + logits matvecs).
3. The server replaces per-request `spawn_blocking` generation with a
   scheduler-owned decode loop: requests join/leave the active batch at
   token boundaries (continuous batching), streaming callbacks fan out
   per stream.

## Consequences

- Default behavior is unchanged until operators raise the permit count.
- KV memory scales with concurrent sessions (~1.8 MiB per 79-token
  session for the 0.5B eval model; bounded by hard_max_tokens).
- The long-chat gate and queue-order proofs remain valid for the
  default configuration; new proofs are required for N > 1.
- Phase 2 restructures the generation callback tower; the
  stream-lifecycle instrumentation must be preserved or re-proven.

## Evidence gates

- Correctness: token-id parity for concurrent requests vs sequential
  runs of the same prompts (fixed prompts, greedy).
- Throughput: `ferrite-openai-throughput --concurrency {1,2,4}`
  aggregate tok/s and requests/s vs the sequential baseline, recorded
  as benchmark notes; no regression at concurrency 1.
- Memory: server RSS peak at concurrency {1,2,4}.
