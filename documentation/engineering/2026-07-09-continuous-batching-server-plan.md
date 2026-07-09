# Implementation plan: continuous batching in ferrite-server (ADR 0011 phase 2, server half)

Status: planned (engine half is implemented and committed:
`scalar::accept_token_ids_batch`, `Matrix::mul_vec_batch`,
`Matrix::argmax_mul_vec_batch`, CLI `--benchmark-batch-streams`).

## Goal

HTTP streams share weight passes: N active generations decode one token
per batched step instead of N independent sessions contending on the
rayon pool. Evidence target: aggregate req/s at client concurrency 4-8
clearly above the concurrent-sessions phase-1 numbers
(1.83 req/s / 88 tok/s aggregate), approaching the CLI batched-engine
numbers (115.9 tok/s at batch 4, 137.7 at batch 8).

## Design (slices)

1. `--experimental-batched-decode` + `--max-batch-streams N` server
   flags (both required to activate; default off keeps today's
   concurrent-sessions path bit-for-bit).
2. `BatchScheduler` (new module in ferrite-server): one dedicated
   `std::thread` owning active jobs. Job = { session (prefilled),
   next_token_id, remaining_budget, piece decoder state
   (TokenTextBuffer equivalent), bounded mpsc sender, stop filter,
   lifecycle handle }. Incoming jobs arrive on an mpsc receiver.
3. Request path when enabled: handler tokenizes + renders prompt as
   today, acquires a batch-admission permit (separate semaphore sized
   `max_batch_streams`), prefills its session via `spawn_blocking`
   (prefill stays per-request initially), then hands the job to the
   scheduler and returns the SSE stream fed by the job's channel.
4. Scheduler loop: drain newly arrived jobs; while jobs active: call
   `accept_token_ids_batch` over all active sessions; per job: decode
   piece, apply stop filter, `blocking_send` the SSE event (a slow
   client backpressures only its own stream — use `try_send` +
   per-job pending buffer, disconnect on channel closed), decrement
   budget, retire finished jobs (EOS / budget / stop / disconnect);
   then poll the receiver for joiners (continuous batching at token
   granularity).
5. Correctness gates: fixed-prompt token-id parity — same prompts via
   default path vs batched path must produce identical completions
   (greedy; engine step is bit-identical per stream, so any divergence
   is a scheduler bug). Existing lifecycle/queue-order proofs remain
   valid for the default path; batched mode gets its own dev-note +
   benchmark note.
6. Throughput evidence: `ferrite-openai-throughput --concurrency
   {2,4,8}` against `--experimental-batched-decode --max-batch-streams
   8`, recorded as a benchmark note; server RSS peak per stream
   (KV grows per session).

## Known integration points

- `stream_generation.rs` currently hands the whole generation to
  `InferenceEngine::generate_with_stage_callbacks_and_cache_options`
  inside one `spawn_blocking`; the batched path bypasses that tower —
  keep the default path untouched and add a parallel entry, then unify
  later.
- Prompt rendering/tokenization helpers and `TokenTextBuffer` live in
  `runtime.rs` (`runtime.rs:435-463`) — expose them (pub(crate)) for
  the scheduler.
- Sessions borrow the model (`ScalarLlamaSession<'a>`); the scheduler
  thread needs an owned handle: either `Arc<InferenceEngine>` + a
  self-referential job created on the scheduler thread (prefill happens
  there too), or make prefill return the session by moving the Arc into
  the thread and keeping sessions local to it. Simplest: scheduler
  thread owns `Arc<InferenceEngine>` and does prefill itself (jobs
  arrive as token ids + params), accepting slightly serialized prefill
  in v1; measure before optimizing prefill admission.

## Follow-ups after v1

- Chunked prefill interleaved with decode steps (TTFT vs aggregate).
- Decode-once batched inner loops for Q6_K/Q4_K; scratch-buffer reuse
  in `accept_token_ids_batch`.
- eval.py: add a batched server phase (client `--concurrency`) so the
  standard eval records batched serving.
