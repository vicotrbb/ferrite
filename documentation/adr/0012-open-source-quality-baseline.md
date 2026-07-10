# ADR 0012: Open Source Quality Baseline

Date: 2026-07-10

Status: Accepted

## Context

Ferrite is becoming a public Rust project with two reusable library crates, two
user-facing binaries, architecture-specific unsafe kernels, and a substantial
performance evidence record. The repository needs an enforceable contract for
compiler reproducibility, public API documentation, dependency safety, CI,
release packaging, and performance regression control.

The research basis is recorded in
`documentation/research/2026-07-10-rust-oss-quality-baseline.md`.

## Decision

1. Pin Rust 1.96.1 for contributors and CI, and declare Rust 1.96 as the
   workspace minimum supported version.
2. Centralize package metadata, internal dependency versions and paths, and
   lint policy in the workspace manifest.
3. Deny Clippy's stable `all` and `cargo` groups, selected production
   restriction lints, undocumented unsafe blocks, and implicit unsafe
   operations inside unsafe functions.
4. Treat rustdoc as part of the public contract. `ferrite-model` and
   `ferrite-inference` deny missing public documentation, and CI denies rustdoc
   warnings across the workspace.
5. Run formatting, strict Clippy, all-target and all-feature tests on Linux and
   macOS. Run documentation, eval-harness, package file-set, advisory, license,
   duplicate, and source-policy checks as separate gates.
6. Keep user and contributor guidance in `docs/`. Keep ADRs, research,
   benchmarks, and completed validation notes in `documentation/`. Do not keep
   transient agent prompts, implementation plans, private tool state, or unused
   binary test assets in the repository.
7. Keep model artifacts outside Git. Retain source-controlled fixtures and eval
   records because they are required for reproducible validation.
8. Accept hot-path changes only after fixed-input performance measurement and
   exact token-trace parity. Code shape alone is not evidence of optimization.
9. Publish `ferrite-model` before `ferrite-inference`, because Cargo verifies
   registry dependencies when preparing the dependent package.

## Consequences

- Public APIs cannot silently gain undocumented items.
- Unsafe changes require a local soundness argument and explicit unsafe block.
- Dependency advisories, unacceptable licenses, unknown sources, duplicate
  versions, and wildcard requirements fail policy checks.
- New contributors get one documented toolchain, validation sequence, and
  performance path.
- Experimental kernels remain explicitly opt-in even when they are faster.
- Repository history remains evidence-rich without retaining transient plans.

## Alternatives Considered

- **Rely on reviewer convention.** Rejected because documentation, unsafe
  invariants, dependency policy, and formatting drift are mechanically
  enforceable.
- **Enable every Clippy pedantic and restriction lint.** Rejected because those
  groups are intentionally context-dependent. Ferrite selects the restrictions
  that protect production behavior and denies all stable default groups.
- **Remove historical eval and benchmark records.** Rejected because they are
  required to detect performance regressions and explain accepted kernel paths.
- **Commit local model files.** Rejected because large third-party artifacts do
  not belong in source control and may have separate licensing terms.

## Evidence

- `rust-toolchain.toml`
- `Cargo.toml`
- `deny.toml`
- `.github/workflows/ci.yml`
- `.github/workflows/security.yml`
- `scripts/check_docs.py`
- `docs/development.md`
- `docs/evaluation.md`
- `documentation/benchmarks/2026-07-10-oss-quality-hardening.md`
