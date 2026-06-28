# Ferrite Goal

You are working in the Ferrite repository.

Ferrite is a CPU-native LLM inference engine written primarily in Rust. The
long-term goal is to build a working, production-grade CPU inference engine that
progressively runs increasingly larger open-weight language models, starting
from tiny Tier 0 models and moving toward 7B-9B models on constrained CPU
hardware.

Ferrite must also be usable as a local model service. A production-grade
OpenAI-compatible HTTP server is a required product milestone, so common
OpenAI clients and curl workflows can target Ferrite with a local base URL in
the same style people use local runtimes such as Ollama. The HTTP surface must
not compromise the core inference goal: server code is generic infrastructure,
while model loading, tokenization, session execution, sampling, and
optimization remain Ferrite-owned.

This is not only an implementation project. Ferrite is also a research project
for CPU inference innovation. Use the existing research corpus as baseline
context, but do not treat it as an implementation contract. Improve it with new
evidence. Stop to research when the next implementation step depends on stale,
uncertain, incomplete, or disputed technical assumptions. Document new research,
new theories, validation results, failed ideas, and architecture decisions.

Core objective:

Build Ferrite through an evidence-driven iteration loop:

1. Inspect the current repository state before acting.
2. Read the relevant baseline research and current documentation.
3. Define the smallest useful implementation or research slice.
4. Document the intended slice when it changes project direction or scope.
5. Implement in Rust with strict engineering standards.
6. Validate correctness with tests, fixtures, reference comparisons, or model
   output checks.
7. Benchmark when performance, memory, latency, or hardware behavior is
   relevant.
8. Fix defects and repeat until the slice meets its stated gate.
9. Record development notes, benchmark notes, ADRs, research notes, and theory
   notes as appropriate.
10. Move to the next model tier only when the current tier's correctness,
    memory, and benchmark gates are documented.

Methodology:

- Work progressively. Do not jump straight to 7B-9B models.
- Start with Tier 0 models from `research/11-testing-model-registry.md`.
- Preserve scalar reference implementations before trusting SIMD, assembly,
  FFI, or platform-specific paths.
- Treat every optimization as a hypothesis until measured.
- Treat every claimed behavior as unproven until verified against current repo
  state or executed evidence.
- Prefer small, reversible implementation slices with strong tests.
- Keep the inference core custom and Ferrite-owned.
- Use normal Rust crates for generic infrastructure such as HTTP, JSON, CLI,
  logging, config, and async runtime when useful.
- Implement core inference machinery from scratch unless an ADR explicitly
  decides otherwise.
- Do not wrap `llama.cpp` as the Ferrite runtime.
- Use external runtimes such as `llama.cpp`, Candle, or mistral.rs only as
  references, comparison baselines, or implementation research unless an ADR
  explicitly allows deeper reuse.

Engineering standards:

- Write production-grade Rust.
- Prefer clear module boundaries, narrow public APIs, and explicit invariants.
- Keep performance-critical code auditable.
- Require deterministic tests for parsers, tensor layout, model config,
  sampling, numerical kernels, and memory behavior.
- Use property tests or fuzz tests for binary parsing and unsafe boundaries
  where practical.
- Enforce formatting, linting, tests, and benchmark gates as the Rust workspace
  matures.
- Do not accept undocumented architecture drift.
- Do not make performance claims without benchmark evidence.
- Do not make correctness claims without tests or reference comparisons.

Unsafe, C, C++, and assembly policy:

- Rust is the default language.
- Unsafe code is allowed only when necessary for mmap, tensor casting, SIMD,
  FFI, assembly-level optimization, or other justified low-level work.
- Every durable unsafe boundary must have a safe public API, documented
  invariants, tests against a safe reference path, and an ADR when the boundary
  is security-relevant or architecture-shaping.
- C, C++, or assembly may be used only for isolated hot paths or experiments
  with a clear reason. Prefer a Rust reference implementation first.

Innovation mandate:

Ferrite should not merely reimplement known approaches. Actively look for CPU
inference improvement opportunities, including but not limited to:

- KV cache layout, compression, quantization, eviction, and sliding windows.
- Memory allocation, mmap, page-cache strategy, streaming weights, and pressure
  handling.
- SIMD, AVX2, AVX-512, NEON, AMX, prefetching, and topology-aware scheduling.
- Quantized formats and CPU-friendly tensor layouts.
- Prefix caching, speculative decoding, and batch-1 or batch-2 execution
  strategies suited to CPUs.
- Model architecture handling that improves correctness, memory use, or
  throughput on CPU.

Document speculative ideas as theory notes before treating them as facts.
Design falsification experiments for theories. Promote only validated ideas
into ADRs or implementation plans.

Documentation requirements:

- `documentation/engineering/` contains operating models, policies, and goal
  prompts.
- `documentation/adr/` contains durable architecture decisions.
- `documentation/dev-notes/` contains concrete implementation and experiment
  logs.
- `documentation/research/` contains focused research updates.
- `documentation/theories/` contains speculative ideas and validation plans.
- `documentation/benchmarks/` contains benchmark protocols and results.

Every meaningful iteration must leave an evidence trail. If no code changes are
made, document the research or planning progress. If code changes are made,
document what changed and how it was validated. If a decision is made, write or
update an ADR.

HTTP API progression:

- Provide an OpenAI-compatible local server as a first-class product path, not
  an afterthought attached to the CLI.
- Start with `GET /health`, `GET /v1/models`,
  `POST /v1/chat/completions`, and `POST /v1/completions`.
- Make non-streaming text generation correct before adding SSE token streaming.
- Keep request/response schemas and endpoint routing in focused server modules;
  do not let HTTP-specific types leak into the inference core.
- Test compatibility with direct HTTP requests and at least one standard
  OpenAI client configured with Ferrite as the base URL.

Model progression:

- Tier 0: 135M-360M models for parser, model plumbing, and deterministic token
  output.
- Tier 1: 0.5B-1.7B models for real matrix sizes, SIMD correctness, and GQA
  variants.
- Tier 2: 3B-4B models for memory management and multi-architecture support.
- Tier 3: 7B-9B models for the main Ferrite target.
- Tier 4: 14B-32B models for streaming and extreme memory-pressure research.

Do not mark a tier complete without documented correctness, memory, and
benchmark evidence scoped to that tier.

Local and homelab safety:

- Development happens on macOS, but Ferrite must target Intel and AMD CPUs as
  well.
- Heavy, risky, host-stressing, or long-running tests should run in a bounded
  homelab pod when possible.
- Use only the Kubernetes `staging` context.
- Before any Kubernetes action, verify the active context.
- Never use any Kubernetes context other than `staging` for Ferrite work.
- Prefer explicit CPU and memory limits for pods.
- Do not run unbounded stress tests, huge downloads, or memory-pressure tests on
  the local Mac without a clear reason.

First concrete milestone:

Ferrite can load a tiny Llama-family GGUF model from Tier 0, parse enough model
metadata and tensors to execute a scalar reference forward path, and produce a
deterministic next-token result that can be compared against a documented
reference.

Before implementing this milestone:

1. Inspect current repository state.
2. Read `documentation/engineering/ferrite-operating-model.md`.
3. Read `documentation/adr/0001-documentation-and-iteration-model.md`.
4. Read `research/11-testing-model-registry.md`.
5. Write or update the relevant development note and ADR if the planned work
   changes project direction.

Completion discipline:

- Keep the long-term Ferrite goal active until the actual engine exists and is
  verified through progressive model tiers.
- Do not redefine success around a smaller subset.
- At the end of each work session, report what changed, what was validated, what
  remains unproven, and the next best slice.
