# ADR 0006: SIMD Unsafe Boundary

Date: 2026-06-27

Status: Accepted

## Context

Ferrite's Tier 1 gate requires SIMD correctness work for real matrix sizes. The
current workspace forbids unsafe code globally with `unsafe_code = "forbid"`,
which was appropriate while the project established a safe scalar reference
path.

Rust CPU intrinsics for AVX2, AVX-512, NEON, and related architecture-specific
kernels require unsafe implementation details. Ferrite's operating model allows
unsafe code only for justified low-level work such as SIMD intrinsics, and it
requires a safe public API, documented invariants, tests against a safe
reference path, and an ADR for durable unsafe boundaries.

Ferrite now has:

- a scalar Llama execution path;
- scalar GQA, RoPE, and KV-cache correctness coverage;
- a matvec reference-check API with relative-error validation; and
- Tier 0 model-output parity policy and evidence.

That is enough structure to define how SIMD may enter the codebase without
weakening the safety model.

## Decision

Ferrite remains safe Rust by default. Unsafe code is permitted only inside
narrow, architecture-specific kernel modules when all of these conditions are
met:

- The module exposes a safe Rust API to the rest of Ferrite.
- Every unsafe block has a local safety comment that states the invariant being
  relied on.
- The module is guarded by target architecture and CPU-feature checks before
  any target-feature function is called.
- A scalar reference implementation remains available for the same operation.
- Tests compare the optimized output against the scalar reference with an
  explicit tolerance. Tier 1 matvec kernels use the 0.1% relative-error gate
  unless a narrower ADR supersedes that threshold for a specific operation.
- The optimized path has a deterministic fallback when the CPU feature is not
  present.
- The module does not own model semantics, tokenizer behavior, prompt handling,
  or reference-output policy. It owns only the low-level numerical kernel.

The first SIMD implementation commit may change the workspace unsafe-code lint
from `forbid` to a locally overridable policy, but that commit must include the
first scoped unsafe module and its reference-comparison tests. Ferrite must not
relax the lint as a standalone configuration change.

Architecture-specific modules should be organized by operation and target, for
example an F32 matvec kernel with scalar, x86_64 AVX2, and aarch64 NEON
implementations behind a safe dispatcher. The dispatcher must select an
optimized implementation only after runtime feature detection succeeds.

## Consequences

SIMD work can begin without treating unsafe as a general permission. The scalar
reference path remains the correctness oracle, and optimized kernels cannot make
correctness claims without executable comparison evidence.

The first unsafe SIMD slice will carry more ceremony than a normal safe Rust
slice: lint-policy change, documented invariants, target guards, fallback
behavior, focused tests, and a development note. That overhead is intentional
because the boundary is durable and performance-critical.

The current codebase still contains no unsafe code after this ADR. This decision
only defines the conditions under which unsafe SIMD may be introduced later.

## Alternatives Considered

Keep `unsafe_code = "forbid"` permanently.

This was rejected because Rust architecture intrinsics require unsafe
implementation boundaries, and Tier 1 explicitly requires SIMD correctness.
Keeping a permanent forbid would block the project from its stated goal.

Relax the unsafe-code lint immediately.

This was rejected because a standalone lint relaxation would weaken the safety
model without adding a reviewed unsafe boundary. The lint should change only in
the same slice that introduces a scoped, tested SIMD module.

Use an external runtime for SIMD kernels.

This was rejected for the Ferrite inference core. External runtimes remain
valid references, but Ferrite's core execution and kernel strategy must remain
Ferrite-owned unless a future ADR explicitly decides otherwise.

## Evidence

- `documentation/engineering/ferrite-operating-model.md` defines the unsafe
  policy and Tier 1 SIMD correctness requirement.
- `documentation/engineering/tier0-gate-status.md` records Tier 0 completion
  and identifies SIMD/GQA validation as the next Tier 1 direction.
- `documentation/adr/0003-scalar-reference-inference-boundary.md` establishes
  the scalar reference path as the oracle for optimized work.
- `documentation/adr/0005-reference-parity-policy.md` requires optimized CPU
  kernels to compare against Ferrite's scalar reference path.
- `documentation/dev-notes/2026-06-27-tier1-matvec-kernel-check.md` records the
  executable matvec reference-comparison harness for future optimized kernels.
