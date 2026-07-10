# ADR 0007: Q8_K activation dot path

Date: 2026-06-28

Status: Accepted as an experimental policy

## Context

The original aarch64 `Q4_K` and `Q6_K` kernels dotted quantized weights against
F32 activations. Repeated weight unpacking limited the integer-dot instruction
opportunities used by established fast quantized kernels. Quantizing activation
blocks to `Q8_K` offered a lower-cost arithmetic path, but it could change
rounding and final argmax decisions.

## Decision

Ferrite provides an internal activation-side `Q8_K` representation and scalar
reference arithmetic for eligible `Q4_K` and `Q6_K` matrix-vector operations.
Architecture-specific implementations can use NEON, DotProd, or I8MM only
behind safe dispatch and runtime feature detection.

The path is governed by an explicit execution policy:

- the default policy retains the exact compatibility path;
- parity-scoped `Q8_K` routing is experimental and opt-in;
- projection roles can be selected independently for diagnostics;
- comparison mode evaluates a candidate without changing default execution;
- residual two-pass activation kernels remain a distinct experimental policy.

Activation block logic and format-specific adapters stay in focused modules.
The public matrix input remains `&[f32]`.

## Consequences

Activation quantization overhead must be amortized by faster dot products.
Matrix-level numerical proximity is insufficient because a small logit margin
can change a generated token. Promotion therefore requires scalar arithmetic
checks, architecture-kernel checks, fixed model token traces, and measured
end-to-end performance.

The experimental policy can evolve without weakening the default compatibility
contract. x86_64 behavior stays on its separately tested dispatch paths.

## Alternatives considered

- **Make the quantized activation path the default.** Rejected because observed
  near-tie logits can diverge after chained approximate operations.
- **Repack every weight matrix first.** Deferred because it adds loader and
  storage complexity before proving the arithmetic contract.
- **Import another runtime's kernels.** Rejected because Ferrite owns its core
  model execution and uses external runtimes only as references.

## Evidence

- `crates/ferrite-inference/src/scalar/q8_k.rs` defines activation blocks and
  scalar checks.
- `crates/ferrite-inference/src/scalar/options.rs` defines explicit policies
  and role scoping.
- `crates/ferrite-inference/src/scalar/q8_k_reference_tests.rs` verifies integer
  identities and signed-scale behavior.
- [`0006-simd-unsafe-boundary.md`](0006-simd-unsafe-boundary.md) defines the
  architecture-specific safety boundary.
- [`../performance.md`](../performance.md) defines the token-parity promotion
  rule for residual and activation experiments.
