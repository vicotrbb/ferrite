# ADR 0012: Open source quality baseline

Date: 2026-07-10

Status: Accepted

## Context

Ferrite has publishable model and inference crates, user-facing binaries,
architecture-specific unsafe kernels, and performance-sensitive experimental
policies. A public repository needs an enforceable contract for API quality,
documentation, safety, dependency policy, package contents, reproducibility,
organization, and regression control.

The primary-source research basis is recorded in
[`../engineering/rust-quality.md`](../engineering/rust-quality.md).

## Decision

1. Pin Rust 1.96.1 for contributors and normal CI, declare Rust 1.96 as the
   workspace MSRV, and check Rust 1.96.0 separately.
2. Centralize package metadata, internal dependencies, lints, and compiler
   profiles in the workspace manifest.
3. Deny Clippy's stable `all` and `cargo` groups plus selected restriction
   lints. Promote pedantic rules only after repository-specific review.
4. Deny missing documentation in published crates. Enforce rustdoc links,
   error sections, documentation Markdown, and `must_use` builder returns.
5. Keep unsafe code denied by default and allow it only in narrow
   architecture-specific modules with local safety arguments and safe dispatch.
6. Run formatting, strict Clippy, default and all-feature tests, doctests,
   rustdoc, MSRV, eval-harness tests, package checks, dependency policy, and
   repository hygiene in CI.
7. Keep one `docs/` tree for maintained guides, ADRs, current engineering
   research, and curated benchmark milestones. Keep raw comparable eval output
   in `scripts/evals/`.
8. Do not retain transient plans, specs, session notes, speculative theory
   dumps, private tool state, generated caches, or model binaries.
9. Keep in-memory source fixtures, real-model test hooks, and eval artifacts
   that are required for correctness and performance regression checks.
10. Build release binaries with ThinLTO, one codegen unit, panic abort, and
    stripped symbols after parity and interleaved performance validation.
11. Accept hot-path changes only after fixed-input measurement and exact or
    explicitly scoped token-trace parity.
12. Publish `ferrite-model` before `ferrite-inference`, then tag the exact
    release commit and update the changelog.

## Consequences

Public API and documentation drift fail mechanically. Unsafe changes require a
reviewable local soundness argument. Dependencies and package archives are
policy-checked. New contributors get one maintained route through the project,
while detailed historical experiments remain recoverable from Git.

Release builds take longer because ThinLTO and one codegen unit trade compile
time for smaller optimized artifacts. Panic abort is acceptable because
production panic paths are denied and process-level failure is safer than
attempting to recover an inference service after an invariant violation.

## Alternatives considered

- **Rely on review convention.** Rejected because most quality rules are
  mechanically enforceable.
- **Enable every Clippy pedantic and restriction lint.** Rejected because the
  Clippy project describes those groups as contextual and potentially
  contradictory.
- **Retain every historical note.** Rejected because it obscures current
  behavior and duplicates Git history.
- **Commit model or binary fixtures.** Rejected because large third-party
  artifacts have separate licensing and do not belong in source control.

## Evidence

- `Cargo.toml`, `rust-toolchain.toml`, and `deny.toml` define compiler and
  dependency policy.
- `.github/workflows/ci.yml` and `security.yml` enforce the repository gate.
- `scripts/check_docs.py` and `scripts/check_repo.py` enforce documentation and
  file hygiene.
- [`../benchmarks/2026-07-10-oss-quality-hardening.md`](../benchmarks/2026-07-10-oss-quality-hardening.md)
  records full correctness and performance evidence.
- [`../releasing.md`](../releasing.md) defines package and publication order.
