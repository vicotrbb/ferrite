# Changelog

All notable Ferrite changes are documented here. The project follows semantic
versioning for published crates while it remains in the `0.x` series.

## Unreleased

No unreleased changes.

## 0.1.0 - 2026-07-10

### Added

- Complete user, operator, contributor, library, evaluation, safety, and
  release documentation.
- Reproducible Rust toolchain, strict lint, dependency policy, package, MSRV,
  repository hygiene, and cross-platform CI gates.
- Successful help and version commands for every shipped executable.
- Versioned, reproducible native release archives for macOS arm64 and Linux
  x86_64, with SHA-256 checksums, SPDX SBOMs, and GitHub build attestations.
- An official non-root OCI server image with multi-architecture Linux support,
  image provenance, and SBOM attestations.
- Automated tag-driven release publication, including trusted crates.io
  publishing once the registry trust relationship is configured.

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
