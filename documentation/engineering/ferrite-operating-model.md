# Ferrite Operating Model

## Mission

Ferrite is a CPU-native LLM inference engine written primarily in Rust. It aims
to run progressively larger open-weight language models on commodity CPUs while
researching and validating improvements in CPU inference, memory management,
quantization, KV cache strategy, scheduling, and instruction-set utilization.
Ferrite also needs a local OpenAI-compatible HTTP server so existing client
workflows can call it with a custom base URL, while the inference core remains
Ferrite-owned.

The project is not only an implementation effort. It is a research and
engineering loop whose outputs are working code, verified measurements,
architecture decisions, development notes, research notes, and tested theories.

## Non-Goals

- Do not wrap `llama.cpp` as the Ferrite runtime.
- Do not optimize for GPU inference.
- Do not accept undocumented architecture drift.
- Do not treat speculative theories as implementation requirements until they
  have validation evidence.
- Do not run risky, host-stressing, or resource-heavy tests on the local Mac
  when a homelab pod is the safer target.
- Do not treat OpenAI compatibility as full OpenAI API parity; unsupported
  hosted, multimodal, tool, or administrative APIs must be explicit future
  scope.

## Iteration Loop

Each Ferrite iteration follows this order:

1. Define the question or implementation slice.
2. Read the relevant repo state and baseline research.
3. Document the intended experiment or development slice.
4. Implement the smallest correct step.
5. Validate correctness with tests or reference comparisons.
6. Benchmark when performance or resource usage is relevant.
7. Fix defects and repeat until the slice meets its gate.
8. Record results in development notes and benchmark notes.
9. Write or update ADRs for durable decisions.
10. Promote validated research or theories into implementation plans only after
    evidence supports them.

## Progressive Model Gates

Ferrite scales by model tier, not by ambition alone. The authoritative model
list starts in `research/11-testing-model-registry.md`.

| Gate | Target | Purpose |
| --- | --- | --- |
| Tier 0 | 135M-360M models | Parser, model plumbing, deterministic token output |
| Tier 1 | 0.5B-1.7B models | Real matrix sizes, SIMD correctness, GQA variants |
| Tier 2 | 3B-4B models | Memory management, multi-architecture support |
| Tier 3 | 7B-9B models | Main Ferrite target: 2 vCPU / 6 GB class proof |
| Tier 4 | 14B-32B models | Streaming and extreme memory-pressure research |

No tier is considered complete until the relevant correctness, memory, and
benchmark gates are documented.

## Rust Engineering Standards

Ferrite code must be written as production Rust:

- Prefer simple, explicit modules with narrow public APIs.
- Keep inference-core dependencies minimal and intentional.
- Use generic crates for infrastructure such as HTTP, JSON, CLI parsing,
  logging, and configuration.
- Keep performance-critical inference code auditable and benchmarked.
- Maintain scalar reference implementations before relying on SIMD paths.
- Require deterministic tests for parsers, tensor layout, sampling, model
  configuration, and numerical kernels.
- Require property tests or fuzz tests for binary parsers and unsafe boundaries
  where practical.
- Treat `unsafe` as an architecture decision, not an implementation detail.

## HTTP API Standards

Ferrite's server surface must follow ADR 0008. The initial compatibility
contract is local text generation through:

- `GET /health`
- `GET /v1/models`
- `POST /v1/chat/completions`
- `POST /v1/completions`

Server code should be isolated from inference code. HTTP handlers own protocol
concerns such as JSON schemas, OpenAI-shaped errors, bearer-token policy,
SSE framing, status codes, and request backpressure. Inference crates own
model format, tokenization, session state, sampling, KV cache behavior, and
numeric kernels.

Non-streaming responses must be correct before token streaming is added.
Streaming support must use Server-Sent Events and be verified with clients that
expect OpenAI-style stream chunks.

## Unsafe-Code Policy

Unsafe code is allowed only when it is necessary for memory mapping, tensor
casting, SIMD intrinsics, FFI, or assembly-level performance work.

Every unsafe boundary must have:

- A safe public API.
- A documented invariant.
- Tests against a safe reference path.
- A clear owner module.
- An ADR if the unsafe boundary is durable or security-relevant.

C, C++, or assembly may be used only for isolated hot paths or platform-specific
experiments. A Rust reference implementation must exist unless the experiment is
only measuring feasibility.

## Validation Standards

Correctness claims require evidence:

- Parser behavior must be tested against real fixtures or generated fixtures.
- Numerical kernels must compare against scalar references within explicit
  tolerances.
- Model output claims must compare against a fixed reference runtime, prompt,
  seed, and model artifact.
- Memory claims must include measured RSS or equivalent platform-specific
  evidence.
- Performance claims must include hardware, model, quantization, context,
  thread count, command, and summary statistics.

## Benchmark Standards

Benchmarks are project artifacts, not anecdotes. A benchmark note must include:

- Date and commit or tree state.
- Hardware and OS.
- CPU feature flags relevant to the run.
- Model name, source, format, and quantization.
- Prompt length, generated token count, context length, and thread count.
- Decode throughput, prefill throughput when available, TTFT when available,
  peak RSS when available, and failure modes.
- Comparison baseline when making a comparative claim.

## Research and Theory Standards

Research notes refine the project baseline with new evidence. Theory notes are
allowed to be speculative, but they must be labeled as hypotheses until tested.

Each theory note must include:

- The hypothesis.
- Why it could improve CPU inference.
- The expected measurement.
- The smallest falsification experiment.
- Known risks and reasons it may fail.

Validated theories can become ADRs or implementation plans. Invalidated
theories remain documented so the project does not repeat the same dead end.

## Homelab Safety

Local development happens on macOS, but Ferrite targets Intel and AMD CPUs as
well. Heavy, risky, or host-stressing tests should run in the homelab when
possible.

Rules:

- Use only the Kubernetes `staging` context.
- Verify the active context before any Kubernetes action.
- Never use another Kubernetes context for Ferrite work.
- Prefer bounded pods with explicit CPU and memory limits.
- Do not run unbounded stress tests, memory pressure tests, or large model
  downloads on the local Mac without an explicit reason.

## First Milestone

The first implementation milestone is correctness-first:

Ferrite can load a tiny Llama-family GGUF model from Tier 0, parse enough model
metadata and tensors to execute a scalar reference forward path, and produce a
deterministic next-token result that can be compared against a documented
reference.

Performance work starts after this correctness path exists.
