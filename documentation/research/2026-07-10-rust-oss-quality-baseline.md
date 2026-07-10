# Rust OSS Quality Baseline

Date: 2026-07-10

## Question

Which current Rust practices should define Ferrite's public repository,
library API, safety policy, dependency policy, and continuous integration
baseline?

## Primary Sources

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) for API
  naming, traits, documentation, interoperability, predictability, and
  future-proofing.
- [Cargo workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
  for shared package metadata, dependency declarations, and lint policy.
- [Cargo `rust-version`](https://doc.rust-lang.org/cargo/reference/rust-version.html)
  for an explicit minimum supported Rust version contract.
- [Clippy documentation](https://doc.rust-lang.org/stable/clippy/) for the
  stable correctness, suspicious, style, complexity, and performance lint
  groups.
- [Rustdoc guidance](https://doc.rust-lang.org/stable/rustdoc/write-documentation/what-to-include.html)
  for crate, module, item, error, panic, and safety documentation.
- [Rustonomicon safety boundary guidance](https://doc.rust-lang.org/nomicon/safe-unsafe-meaning.html)
  for keeping unsafe code narrow and making every safety obligation explicit.
- [Cargo continuous integration guidance](https://doc.rust-lang.org/stable/cargo/guide/continuous-integration.html)
  for formatting, lint, build, test, and documentation checks.
- [RustSec](https://rustsec.org/) and
  [`cargo-deny`](https://embarkstudios.github.io/cargo-deny/) for advisory,
  license, source, and dependency graph policy.
- [Rust 1.96.1 release listing](https://blog.rust-lang.org/releases/latest/)
  for the pinned project toolchain used when this baseline was recorded.

## Decisions

1. Pin Rust 1.96.1 for contributors and CI, and declare Rust 1.96 as the
   workspace `rust-version` contract.
2. Centralize internal crate versions and paths, package metadata, Rust lints,
   and Clippy lints in the workspace manifest.
3. Deny Clippy's stable `all` and `cargo` groups, plus selected restriction
   lints that protect production code. Do not enable the entire `pedantic` or
   `restriction` groups, because Clippy documents those groups as intentionally
   noisy and context dependent.
4. Deny undocumented unsafe blocks and unsafe operations that are implicit
   inside unsafe functions. Every unsafe block must state the local invariant
   that makes it sound.
5. Treat rustdoc as part of the public API. Public library crates must provide
   crate, module, item, error, panic, and safety documentation where relevant.
6. Keep lockfile-aware formatting, lint, test, rustdoc, documentation, advisory,
   license, and dependency-source checks in CI.
7. Measure performance changes against a fixed model, prompt, token budget,
   policy, machine, and token trace. An optimization is not accepted on code
   shape alone.
8. Keep durable ADRs, research, benchmarks, and validation notes. Remove
   transient implementation plans, agent prompts, and unused binary assets.

## Ferrite Application

The repository applies this baseline through `rust-toolchain.toml`, workspace
metadata and lint tables in `Cargo.toml`, `deny.toml`, GitHub Actions workflows,
the contributor and security policies, the maintained `docs/` tree, and the
evaluation harness. The baseline is intentionally enforceable, not advisory.
