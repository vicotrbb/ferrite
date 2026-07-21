# Changelog

All notable Ferrite changes are documented here. The project follows semantic
versioning for published crates while it remains in the `0.x` series.

## Unreleased

### Compatibility

- Public Phi-3 architecture variants and additional matrix storage variants
  require the next pre-1.0 release to advance from `0.2` to `0.3`. A `0.2.x`
  release would violate the published exhaustive enum contracts and is rejected
  by `cargo semver-checks`.

### Changed

- Migrated the workspace to the Rust 2024 edition and Cargo resolver 3, with
  explicit scheduler and channel-sender destruction order at migration-sensitive
  boundaries.
- Consolidated the server's artifact-gated integration cases into explicit
  test harnesses, reducing its Cargo targets from 43 to 11 without removing a
  test case.
- Reused the existing token buffer allocation when applying BPE merges, while
  preserving the exact tokenizer output contract.
- Reused one normalization scratch buffer across every transformer layer in a
  single-stream token step, eliminating repeated layer-local allocations while
  preserving the exact generated token trace.
- Applied rotary position encoding inside existing query and key buffers,
  eliminating per-head replacement allocations in single and batched token
  steps while preserving the exact generated token trace.
- Applied the canonical Rust 2024 rustfmt style across the workspace.

### Documentation

- Updated the Rust quality baseline, development workflow, portability gates,
  and real-model test commands for the current edition and harness layout.
- Clarified the three published crate boundaries and recorded the in-place RoPE
  allocation and parity diagnostic without promoting contaminated-host timing.

## 0.2.0 - 2026-07-13

### Added

- A shared read-only GGUF mapping API that lets validated quantized tensors
  retain file ranges without a model-sized heap copy.
- Context-only single-session and batched prefill paths for non-final prompt
  tokens whose output logits are not observable.
- Exact-prompt cohort fan-out through independent KV snapshots in the
  continuous scheduler.
- Eval schema version 3 with model SHA-256 records, complete ordered server
  token-ID traces, and whole-cohort parity checks.

### Changed

- The CLI and server now load quantized weights from immutable mapped model
  storage while retaining owned loading for library callers.
- Continuous batching now uses a bounded five-millisecond admission window,
  batched context-only prompt evaluation, and generic equal-token-sequence
  prompt grouping.
- Batched Q8 output argmax reuses per-worker scratch storage instead of
  allocating a vector for every vocabulary row.
- Architecture, server, CLI, evaluation, performance, and release guidance now
  document mapped-file safety and the shared-prompt execution contract.

### Performance

- Raised the repeated four-request Qwen2.5 0.5B Q4_K_M shared-prompt server
  median from 93.21 to 131.45 aggregate tokens per second, a 41.03% increase.
- Reached a repeated eight-request median of 159.58 aggregate tokens per second.
- Reduced four-request server peak RSS from 956.8 MiB to 568.8 MiB and retained
  CLI RSS from 1,005.1 MiB to 556.7 MiB.
- Reduced the four-request median time to first token from 880 ms to 183 ms.

### Validation

- Verified every response in the accepted request cohorts against complete
  ordered token-ID traces and exact default-route parity.
- Passed strict all-feature Rust checks, default and all-feature tests, rustdoc,
  x86_64 Linux cross-checks, package verification, dependency policy, and
  reproducible release-tool gates.

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
- Source-distributed integration-test fixtures that remain package-verifiable
  alongside the public Rust libraries.

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
