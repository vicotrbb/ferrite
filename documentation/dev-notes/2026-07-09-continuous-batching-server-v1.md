# Continuous-batching server v1

- Date: 2026-07-09
- Scope: experimental streaming HTTP scheduler for ADR 0011 phase 2
- Default behavior: unchanged

## Activation

Both flags are required:

```sh
ferrite-server \
  --experimental-batched-decode \
  --max-batch-streams 4 \
  --model target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf
```

The batched path currently serves streaming chat and legacy completion
requests when prefix caching and prompt-cache tracing are disabled. Other
requests deliberately fall back to the existing inference path.

## Runtime shape

One dedicated scheduler thread owns every active `ScalarLlamaSession` and the
model borrow. Requests submit rendered prompts through a bounded channel and
receive bounded per-stream generation events. At each token boundary the
scheduler admits joiners, collects only streams that are ready, calls
`accept_token_ids_batch`, and fans the results back out. A full or disconnected
client channel pauses or retires only that stream; it does not block ready
peers. A single ready stream takes the normal exact decode path so sparse
traffic does not pay the batch engine's allocation and dispatch overhead.

A separate semaphore bounds admitted batched streams. Its permit is held by
the SSE forwarding task and is released on completion, error, or client
disconnect. The legacy inference semaphore remains independent.

## Correctness gates

- Two parallel four-token fixture streams match the default path's visible
  SSE content, finish reason, and usage exactly.
- Dropping a batched response body releases its admission permit.
- Existing default-path server suite remains green (403 unit tests plus the
  HTTP and client integration suites).
- Strict `cargo clippy -p ferrite-server --all-targets -- -D warnings` passes.

## Evaluation harness

`scripts/eval.sh --server-batch-streams N --requests M` now records a second
server phase. It reports client concurrency and aggregate completion tok/s as
`requests/s * generated tokens`, while retaining first-stream TTFT and token
latency separately.

The first integrated artifact,
`scripts/evals/2026-07-09-221222-qwen2.5-0.5b-instruct-q4_k_m.json`, was run at
load averages 5.77/10.06/9.94. It observed 55.61 aggregate tok/s for batching
versus 35.0 for the legacy server, but the same run's engine batch-4 result
collapsed from the previous clean 103.58 tok/s to 30.25 tok/s. The artifact is
therefore diagnostic proof of end-to-end execution, not retained absolute
performance evidence. A quiet-machine rerun is required before closing the
throughput gate.

The accepted combined rerun,
`scripts/evals/2026-07-09-235740-qwen2.5-0.5b-instruct-q4_k_m.json`, records:

- engine batch 2: 102.58 aggregate tok/s;
- engine batch 4: 128.14 aggregate tok/s;
- engine batch 8: 149.29 aggregate tok/s;
- continuous HTTP batch 4: 87.46 aggregate tok/s versus 79.61 for the
  residual-I8MM sequential HTTP phase in the same run;
- exact stream-0 parity for every engine batch and HTTP token-ID parity
  between the sequential and continuously batched responses.

The residual single-stream policy and exact continuous-batch policy are
separate opt-ins. `ferrite-server` rejects enabling both in one process until
the batch kernels implement residual activation dispatch, avoiding a silent
mixed-arithmetic session.

## Follow-ups

- Interleave or chunk prompt prefill so a new long prompt cannot pause active
  decode streams.
- Reuse batch scratch buffers in `accept_token_ids_batch`.
- Add residual activation dispatch to batched kernels, then remove the
  mutually exclusive server guard.
