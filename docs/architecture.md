# Architecture

Ferrite separates file-format concerns, inference mechanics, serving, command
tools, and fixtures into small workspace crates.

## Request and inference flow

```text
read-only GGUF mapping
  -> ferrite-model parses metadata, tensor ranges, and tokenizer
  -> architecture adapter normalizes GGUF layouts into common transformer weights
  -> ferrite-inference validates model shape and retains zero-copy byte ranges
  -> ScalarTransformerSession owns KV state and evaluates prompt tokens
  -> architecture dispatch selects portable, AVX2, NEON, or I8MM kernels
  -> the request sampler or bounded grammar selects the next token
  -> CLI records it, or the server maps it to JSON, tool calls, or SSE
```

## Workspace crates

### `ferrite-model`

Owns bounded GGUF parsing, stable architecture layout descriptors, tensor
metadata, BPE and SentencePiece tokenization, and the shared read-only
model-file mapping. It does not execute model math.

### `ferrite-inference`

Owns retained matrix representations, model loading, sessions, attention,
RoPE, RMS normalization, SwiGLU, quantized matrix-vector kernels, threading,
prefix-cache identities, snapshots, and optional Locus KV storage.

Loader adapters split architecture-specific fused tensors, then construct one
architecture-neutral execution model. The historical Llama-named public types
remain as compatibility aliases while new code can use the stable transformer
names.

The default session is single-owner and keeps mutable KV state out of shared
global structures. Model weights are immutable and can be shared by sessions.
F16, BF16, and supported quantized matrices retain validated ranges of one
shared GGUF mapping. F32 matrices and required vectors use owned storage.
Token embedding lookup decodes only the quantization blocks that intersect one
selected row, including layouts where a block spans row boundaries.

### `ferrite-server`

Owns configuration, HTTP routing, OpenAI request validation, authentication,
CORS, backpressure, streaming lifecycle, prefix-cache coordination,
continuous-batch scheduling, bounded JSON grammar, function-call parsing,
Responses compatibility, throughput clients, and long-chat gates. Blocking
model work runs outside the async request executor where required. Parsed tool
calls are response data only and have no execution path inside the server.

### `ferrite-cli`

Owns local generation, verified built-in model acquisition, deterministic token
checks, profiling, and benchmark output. It is also the process measured by
the eval harness.

### `ferrite-fixtures`

Builds minimal GGUF fixtures in memory. It prevents test correctness from
depending on committed binary assets or network downloads.

## Kernel dispatch

Portable implementations define correctness. One capability boundary detects
CPU features, and the selected provider decides whether an operation may enter
an optimized module. `auto` can choose a proven NEON, DotProd, I8MM, or AVX2
kernel only when its runtime feature is present. `portable` can only disable
optimized entries; it cannot force unsupported instructions. Batch members
must share one provider and complete execution policy.

Unsafe code is limited to architecture-specific kernels and the read-only
file-mapping boundary. Each allowance has a reason, every unsafe block
documents its preconditions, and safe kernel wrappers validate shapes and CPU
features before entry. Mapping a file requires the application boundary to
guarantee that the artifact is not modified or truncated while retained ranges
are live. See [CPU portability and dispatch](portability.md).

Optimized kernels must preserve the required accumulation order or pass an
explicit numerical and token-parity gate. An optimization is not accepted from
inspection alone.

## Scheduling

The normal server path admits sampling policies that require full logits
through a semaphore and runs each model session to completion or cancellation.
Experimental continuous batching owns the fused-greedy path for streaming and
non-streaming responses. Concurrent arrivals are coalesced for a bounded
five-millisecond admission window. Each request first acquires an immutable
lease on its longest compatible prefix snapshot, then only uncached prompt
tokens are prefilled through the weight-sharing batch kernels. Exact duplicate
prompts are evaluated once when their cache namespace and options match, then
restored into independent mutable sessions. Locus restores borrowed snapshot
rows directly into each mapped pool, avoiding a second prompt-sized heap copy
for every duplicate session. A single-session prompt skips the output
projection for non-final prompt tokens because those intermediate logits are
not observable.

Scheduler admission, its FIFO waiting queue, and each stream's response events
are independently bounded. Every ready stream advances one token per scheduler
cycle. Backpressured streams pause without preventing other ready streams from
advancing, and disconnected receivers are retired during admission, prefill, or
at a decode boundary.

The optional Locus backend stores each session in fixed-size mapped blocks with
an explicit token cap and stale-handle protection. Prefix-cache snapshots have
separate entry and byte budgets. Cache values use reference-counted immutable
ownership so eviction is safe while a request holds a restore lease. Restored
sessions remain single-owner and mutable, which avoids cross-request KV races.

Kernel provider, activation policy, KV backend, model, tokenizer, chat template,
and namespace all participate in cache compatibility. State produced by one
execution contract cannot be restored into another.

## Durable decisions and evidence

Architecture decisions live under [`docs/adr/`](adr/README.md). Curated
measured claims live under [`docs/benchmarks/`](benchmarks/README.md), and raw
eval output lives under `scripts/evals/`.
Transient implementation plans and private tool state are not repository
artifacts.
