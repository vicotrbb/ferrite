# Changelog

All notable Ferrite changes are documented here. The project follows semantic
versioning for published crates while it remains in the `0.x` series.

## Unreleased

### Added

- Complete user, operator, contributor, library, evaluation, safety, and
  release documentation.
- Reproducible Rust toolchain, strict lint, dependency policy, package, MSRV,
  repository hygiene, and cross-platform CI gates.
- Successful help and version commands for every shipped executable.

### Changed

- Curated project documentation into one `docs/` tree with durable ADRs,
  current research, and milestone benchmark evidence.
- Hardened x86_64 AVX2 boundaries with explicit safety arguments,
  architecture-scoped batching helpers, and feature-aware fallback tests.
- Tuned release builds with ThinLTO, one codegen unit, panic abort, and stripped
  symbols after token-parity and interleaved performance checks.

### Removed

- Transient plans, session logs, speculative theory files, superseded research
  roadmaps, private tool state, and redundant benchmark scratch output.

## 0.1.0

Initial alpha development version. Not yet published.
