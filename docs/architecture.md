# Architecture

Ferrite separates file-format concerns, inference mechanics, serving, command
tools, and fixtures into small workspace crates.

## Request and inference flow

```text
read-only GGUF mapping
  -> ferrite-model parses metadata, tensor ranges, and tokenizer
  -> ferrite-inference validates model shape and retains zero-copy quantized ranges
  -> ScalarLlamaSession owns KV state and evaluates prompt tokens
  -> architecture dispatch selects portable, AVX2, NEON, or I8MM kernels
  -> greedy argmax selects the next token
  -> CLI records it, or the server maps it to JSON or SSE
```

## Workspace crates

### `ferrite-model`

Owns bounded GGUF parsing, architecture-aware configuration, tensor metadata,
tokenization, and the shared read-only model-file mapping. It does not execute
model math.

### `ferrite-inference`

Owns retained matrix representations, model loading, sessions, attention,
RoPE, RMS normalization, SwiGLU, quantized matrix-vector kernels, threading,
prefix-cache identities, snapshots, and optional Locus KV storage.

The default session is single-owner and keeps mutable KV state out of shared
global structures. Model weights are immutable and can be shared by sessions.
Quantized matrices retain validated ranges of one shared GGUF mapping; dense
tensors are decoded into owned F32 storage.

### `ferrite-server`

Owns configuration, HTTP routing, OpenAI request validation, authentication,
CORS, backpressure, streaming lifecycle, prefix-cache coordination,
continuous-batch scheduling, throughput clients, and long-chat gates. Blocking
model work runs outside the async request executor where required.

### `ferrite-cli`

Owns local generation, deterministic token checks, profiling, and benchmark
output. It is also the process measured by the eval harness.

### `ferrite-fixtures`

Builds minimal GGUF fixtures in memory. It prevents test correctness from
depending on committed binary assets or network downloads.

## Kernel dispatch

Portable implementations define correctness. Optimized modules are selected by
compile-time architecture and runtime feature detection. Unsafe code is limited
to architecture-specific kernels and the read-only file-mapping boundary. Each
allowance has a reason, every unsafe block documents its preconditions, and
safe kernel wrappers validate shapes and CPU features before entry. Mapping a
file requires the application boundary to guarantee that the artifact is not
modified or truncated while retained ranges are live.

Optimized kernels must preserve the required accumulation order or pass an
explicit numerical and token-parity gate. An optimization is not accepted from
inspection alone.

## Scheduling

The normal server path admits generation through a semaphore and runs a model
session to completion or cancellation. Experimental continuous batching owns a
scheduler that advances several streaming sessions together so matrix weights
can be reused within each decode step. Concurrent arrivals are coalesced for a
bounded five-millisecond admission window and their non-final prompt tokens are
prefilled through the same weight-sharing batch kernels. Exact duplicate
prompts are evaluated once, then copied into independent sessions through a
validated KV snapshot. A single-session prompt skips the output projection for
non-final prompt tokens because those intermediate logits are not observable.

Prefix caching and continuous batching remain separate experimental contracts.
Cache-enabled requests use the normal path.

## Durable decisions and evidence

Architecture decisions live under [`docs/adr/`](adr/README.md). Curated
measured claims live under [`docs/benchmarks/`](benchmarks/README.md), and raw
eval output lives under `scripts/evals/`.
Transient implementation plans and private tool state are not repository
artifacts.
