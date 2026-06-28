# ADR 0007: Q8_K Activation Dot Path

Date: 2026-06-28

Status: Accepted

## Context

Tier 1 throughput remains incomplete. The latest Qwen2.5-1.5B Q4_K_M
benchmark-token profile shows hot time in Q4_K FFN gate/up, Q4_K/Q6_K FFN down,
and Q6_K output projection roles. The current Ferrite aarch64 Q4_K and Q6_K
NEON kernels dot GGUF quantized weights directly against `f32` activation
vectors. That shape is correctness-friendly, but it requires repeated unpacking
of low-bit weights into float lanes and cannot use the integer dot arithmetic
used by the relevant llama.cpp Q4_K/Q6_K paths.

The current research note records three possible directions:

- local cleanup of the existing `f32` kernels;
- an activation-side `q8_K` dot path;
- repacked/interleaved row layouts.

Thresholded Q4_K/Q6_K row-level scheduling was already tested and rejected, so
the next optimization must change the arithmetic contract or data layout rather
than retrying the scheduler shape.

## Decision

Ferrite will add an internal activation-side `q8_K` dot path for Q4_K and Q6_K
matvecs before attempting repacked row layouts.

The initial design is:

- keep the public matvec input as `&[f32]`;
- quantize each eligible 256-value activation segment into an internal
  `BlockQ8K` representation;
- implement scalar Q4_K x Q8_K and Q6_K x Q8_K dot helpers as the correctness
  contract for the new arithmetic;
- add aarch64 NEON helpers only after the scalar contract is tested;
- keep public Q4_K/Q6_K dispatch on the existing exact/scoped SIMD paths until
  a target-specific Q8_K helper path is tested against scalar Q8_K adapters,
  existing reference-comparison gates, and real model-output checks;
- represent the route as an explicit execution policy: default execution stays
  on the existing paths, while the Q8_K route is allowed only under the
  experimental parity-scoped policy;
- keep x86_64 AVX2 behavior unchanged until a separate tested slice adds a
  matching optimized path.

The implementation must stay modular. `q8_K` activation block logic belongs in a
focused module. Q4_K/Q6_K adapter logic belongs in small format-specific modules
instead of growing the existing Q4_K and Q6_K files into broad mixed-purpose
modules.

## Consequences

This path introduces a numerical contract change for optimized Q4_K/Q6_K matvecs
because the activation vector is quantized before the dot product. The scalar
Q8_K adapters are an internal arithmetic contract; they are not a replacement
for Ferrite's decode-to-`f32` scalar reference. Correctness gates must compare
optimized output against the scalar Q8_K adapters, preserve the existing
reference-comparison harness where applicable, and pass real model-output checks
before the path can be treated as usable for Tier 1 evidence.

Activation quantization has overhead. Benchmarks must prove that the overhead is
amortized on real Tier 1 rows before any throughput claim is made.

Repacked row layouts remain a possible future optimization, but they are not in
scope until the `q8_K` activation contract proves correctness and benchmark
value.

## Alternatives Considered

Optimize the current `f32` kernels first.

This is smaller, but it keeps Ferrite on a different arithmetic contract from
the known fast Q4_K/Q6_K reference path. It may still be useful later for
fallbacks or non-eligible shapes, but it is not the next primary Tier 1
hypothesis.

Add repacked Q4_K/Q6_K row layouts first.

This was rejected as the immediate next step because it adds loader/storage
complexity before proving that activation quantization and integer dot
arithmetic improve Ferrite's actual Tier 1 decode path.

Wrap llama.cpp or import its runtime kernels.

This remains outside Ferrite's inference-core boundary. llama.cpp stays a
reference implementation and benchmarking baseline, not the Ferrite runtime.

## Evidence

- `documentation/research/2026-06-28-tier1-q4-q6-kernel-hypothesis.md`
  compares Ferrite's current kernels with llama.cpp's Q4_K/Q6_K x Q8_K paths.
- `documentation/dev-notes/2026-06-28-q8-k-reference-arithmetic.md` audits the
  implemented Path B arithmetic against llama.cpp's generic and ARM NEON Q4_K
  and Q6_K x Q8_K reference paths.
- `documentation/dev-notes/2026-06-28-q8-k-activation-policy.md` records the
  explicit default-only versus experimental parity-scoped execution policy.
- `documentation/dev-notes/2026-06-28-q8-k-row-shape-guardrail.md` records the
  explicit whole-K-block row-shape invariant for scalar and aarch64 NEON Q8_K
  row adapters.
- `documentation/dev-notes/2026-06-28-q8-k-empty-activation-guardrail.md`
  records the non-empty activation block collection invariant for
  `BlockQ8K::quantize_blocks`.
- `documentation/dev-notes/2026-06-28-q8-k-neon-signed-scale-guardrail.md`
  records target-specific aarch64 NEON parity against the scalar Q8_K helpers
  for both signed activation-scale polarities.
- `documentation/dev-notes/2026-06-28-q8-k-benchmark-compare-guardrail.md`
  records CLI coverage for benchmark-token Q8_K comparison output.
- `documentation/benchmarks/2026-06-28-tier1-qwen2-1-5b-current-profile.md`
  records the current Qwen2.5-1.5B profile and hot aggregate roles.
- `documentation/dev-notes/2026-06-28-tier1-q4-q6-thresholded-row-parallel-regression.md`
  records the rejected Q4_K/Q6_K scheduling experiment.
- `documentation/adr/0003-scalar-reference-inference-boundary.md` keeps the
  scalar path as the optimized-kernel oracle.
- `documentation/adr/0006-simd-unsafe-boundary.md` defines the safe API,
  target-feature, fallback, and reference-test requirements for SIMD kernels.
