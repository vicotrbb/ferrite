# OSS Quality Hardening

Date: 2026-07-10

## Goal

Prepare Ferrite for public collaboration without changing inference behavior:
complete user and public API documentation, enforce current Rust quality and
safety practices, remove transient repository artifacts, fix dependency risks,
and accept only measured low-overhead changes.

## Changes

- Added a maintained `docs/` tree covering setup, CLI, server, OpenAI
  compatibility, models, architecture, safety, evaluation, performance,
  development, and troubleshooting.
- Added a golden performance path and explicit experimental-policy boundaries.
- Added `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, issue forms, a
  pull request template, Dependabot, Linux and macOS CI, package checks, and a
  scheduled dependency-policy workflow.
- Pinned Rust 1.96.1, declared Rust 1.96 as the workspace minimum, centralized
  package metadata and internal dependencies, and removed the sibling-checkout
  requirement from the optional Locus feature.
- Added strict workspace Rust and Clippy policy, explicit unsafe blocks, safety
  comments, and reasons for all lint allowances.
- Completed and enforced rustdoc for every public item in `ferrite-model` and
  `ferrite-inference`.
- Updated `crossbeam-epoch` from 0.9.18 to 0.9.20 to resolve RUSTSEC-2026-0204.
- Added advisory, license, source, duplicate, and wildcard dependency policy in
  `deny.toml`.
- Removed implementation plans, design specs, and the agent goal prompt after
  preserving their durable outcomes in ADRs, source, tests, and evidence notes.
- Preserved generated fixtures, real-model test hooks, and eval artifacts.
- Audited the production causal-attention path and confirmed that it already
  reuses one score allocation across heads and normalizes it in place. No
  speculative runtime rewrite was retained without isolated evidence.
- Tested a one-allocation, in-place RoPE implementation. It preserved the exact
  token trace but reduced throughput 4.17% by median, so it was reverted and
  recorded as a rejected experiment.
- Added conventional successful `--help` and `-h` behavior for both binaries,
  with integration tests.
- Made eval release builds lockfile-aware and prevented generated Markdown from
  introducing em dashes.

## Validation

- Rust 1.96.1 formatting, strict Clippy, workspace rustdoc, and all-target,
  all-feature tests passed.
- The complete available workspace suite passed 760 tests with no failures;
  68 artifact-dependent tests remained explicitly ignored.
- Sixteen ignored-by-default real Qwen2.5-0.5B client, catalog, HTTP,
  concurrency, streaming, and stop-sequence tests passed when supplied the
  local model artifact.
- `cargo audit`, `cargo deny`, `cargo machete`, and duplicate dependency checks
  passed.
- `ferrite-model` packaged and verified. The `ferrite-inference` archive file
  set passed inspection, with full package verification correctly deferred
  until the matching model crate version is published.
- Documentation checks passed across every Markdown file, with no em dashes,
  broken maintained links, or tracked transient plan/spec directories.
- Performance and token-parity evidence is recorded in
  `documentation/benchmarks/2026-07-10-oss-quality-hardening.md`.
