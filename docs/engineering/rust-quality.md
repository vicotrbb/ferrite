# Rust quality baseline

Date: 2026-07-10

Updated: 2026-07-16

## Purpose

This baseline defines Ferrite's enforceable Rust API, safety, dependency,
documentation, release, and optimization standards. It uses primary project
sources and separates stable guidance from context-dependent lint opinions.

## Primary sources

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) for
  naming, traits, predictability, error design, metadata, and documentation.
- [Cargo workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
  for shared package metadata, dependency declarations, profiles, and lints.
- [Rust edition migration](https://doc.rust-lang.org/edition-guide/editions/transitioning-an-existing-project-to-a-new-edition.html),
  [tail-expression scope](https://doc.rust-lang.org/edition-guide/rust-2024/temporary-tail-expr-scope.html),
  and [`if let` scope](https://doc.rust-lang.org/edition-guide/rust-2024/temporary-if-let-scope.html)
  for the Rust 2024 migration and explicit destructor ordering.
- [Cargo dependency resolution](https://doc.rust-lang.org/cargo/reference/resolver.html#rust-version)
  for resolver 3 and MSRV-aware dependency selection.
- [Cargo manifest format](https://doc.rust-lang.org/cargo/reference/manifest.html)
  and [publishing guidance](https://doc.rust-lang.org/cargo/reference/publishing.html)
  for package contents, metadata, dependency order, and dry runs.
- [Cargo profiles](https://doc.rust-lang.org/cargo/reference/profiles.html) for
  release optimization, LTO, codegen units, stripping, and panic strategy.
- [Clippy documentation](https://doc.rust-lang.org/stable/clippy/) for stable
  default lint groups and targeted opt-in lints.
- [Rustdoc guidance](https://doc.rust-lang.org/stable/rustdoc/write-documentation/what-to-include.html)
  for crate, module, item, error, panic, safety, and example documentation.
- [Rustdoc lints](https://doc.rust-lang.org/rustdoc/lints.html) for broken links,
  missing crate documentation, and public documentation drift.
- [Rustonomicon safety boundaries](https://doc.rust-lang.org/nomicon/safe-unsafe-meaning.html)
  and [`core::arch`](https://doc.rust-lang.org/stable/core/arch/) for safe
  wrappers, target-feature checks, and explicit unsafe obligations.
- [Rust compiler PGO guide](https://doc.rust-lang.org/rustc/profile-guided-optimization.html)
  for future profile-guided optimization experiments.
- [RustSec](https://rustsec.org/) and
  [`cargo-deny`](https://embarkstudios.github.io/cargo-deny/) for advisory,
  license, source, and dependency graph policy.

## Applied decisions

1. Pin Rust 1.96.1 for reproducible development and CI, declare Rust 1.96 as
   the MSRV, and run an explicit Rust 1.96.0 CI check.
2. Use the Rust 2024 edition and resolver 3. Centralize authorship, edition,
   license, repository, MSRV, internal paths, lints, and release compiler
   settings in the workspace manifest.
3. Deny Clippy's stable `all` and `cargo` groups, plus selected restriction
   lints that prevent production panics, debug leftovers, undocumented unsafe
   blocks, wasteful clones, and other known hazards.
4. Do not enable all `pedantic`, `restriction`, or `nursery` lints. Clippy
   documents those groups as contextual or unstable. Audit them, then promote
   only rules with a clear Ferrite-wide contract.
5. Publishable crates deny missing rustdoc. They also enforce Markdown hygiene,
   error documentation, and `must_use` on builder-style returned replacements.
6. Keep unsafe code inside architecture-specific modules. Safe dispatchers
   validate architecture, CPU features, shapes, lengths, and numeric
   invariants before calling a kernel.
7. Run default and all-feature tests, doctests, strict rustdoc, dependency
   policy, package content, and repository hygiene checks in CI.
8. Inspect crate archives before publication. Model binaries, generated caches,
   private process artifacts, and unrelated benchmark data are forbidden.
9. Build release binaries with optimization level 3, ThinLTO, one codegen unit,
   panic abort, and stripped symbols. The profile was accepted for a 43 percent
   CLI binary size reduction, exact token parity, and no reliable throughput
   regression in interleaved measurements.
10. Keep `target-cpu=native`, fat LTO, PGO, allocator changes, and hot-loop
    rewrites experimental until fixed-input measurements prove a benefit.
11. Accept runtime optimization only with repeated-run statistics, exact or
    explicitly scoped token parity, and measured TTFT, throughput, memory, CPU,
    and tail latency.
12. Keep Cargo integration targets bounded. Group artifact-gated real-model
    cases into one explicit harness so routine checks compile shared server and
    support code once, without deleting or weakening model coverage.

## Repository enforcement

The baseline is implemented through `rust-toolchain.toml`, workspace manifests,
crate-level lints, release profiles, `deny.toml`, GitHub Actions,
`scripts/check_docs.py`, `scripts/check_repo.py`, package checks, and the eval
harness. See [ADR 0012](../adr/0012-open-source-quality-baseline.md) for the
baseline, [ADR 0020](../adr/0020-rust-2024-and-bounded-test-harnesses.md) for
the current edition and test layout, and [evaluation](../evaluation.md) for
exact commands.
