# ADR 0011: Concurrent serving and batched decode

Date: 2026-07-09

Status: Accepted and implemented as opt-in policies

## Context

A single request does not always saturate host memory bandwidth, while fully
independent concurrent sessions reread the same model weights and compete on
the shared worker pool. Ferrite needed separate controls for simple concurrent
admission and weight-sharing continuous batching.

## Decision

The loaded inference engine is immutable and shared through `Arc`. Prefix-cache
state owns its own synchronization. Normal generation has a configurable
`--max-concurrent-inferences` permit count, with a default of one to preserve
serial admission and backpressure.

The inference crate also provides batched matrix operations and a batched token
step. Each stream retains its own session, attention state, KV cache, stop
logic, and output. Supported kernels stream one weight row across several
activation vectors while preserving each stream's accumulation order.

The server's experimental scheduler owns continuous streaming batches. Requests
are coalesced through a bounded admission window, non-final prompt tokens use
context-only batched steps, exact duplicate prompts fan out through independent
KV snapshots, streams join and leave decode at token boundaries, and
per-stream outputs are fanned back to the existing SSE lifecycle.
`--experimental-batched-decode` and `--max-batch-streams` are both required.

Prefix-cache requests, trace-enabled requests, non-streaming requests, and the
residual activation policy remain outside the continuous-batch contract and
use or require the normal path.

## Consequences

Raising independent permits can improve aggregate request rate while reducing
per-stream latency consistency. Continuous batching can improve aggregate
decode throughput, but adds queue, cancellation, disconnect, admission, and
fairness responsibilities.

Default behavior remains serial. Operators must choose latency or aggregate
throughput based on machine-specific evidence.

## Alternatives considered

- **Always run independent concurrent sessions.** Rejected because it cannot
  reuse weight reads and can amplify bandwidth contention.
- **Enable batching by default.** Rejected while the scheduler is experimental
  and model plus platform evidence is still narrow.
- **Share mutable session state across requests.** Rejected because isolation
  and cancellation are simpler with one session per stream.

## Evidence

- `crates/ferrite-inference/tests/batched_decode.rs` proves per-stream token
  equivalence, context-only prefill equivalence, and model-identity rejection.
- `crates/ferrite-server/src/runtime/scheduler.rs` implements scheduler-owned
  continuous batches.
- `crates/ferrite-server/tests/openai_http.rs` covers disconnect and permit
  release behavior.
- [`../benchmarks/2026-07-09-concurrent-serving-phase1.md`](../benchmarks/2026-07-09-concurrent-serving-phase1.md)
  records phase-one measurements.
- [`../performance.md`](../performance.md) records the current aggregate
  throughput gate.
