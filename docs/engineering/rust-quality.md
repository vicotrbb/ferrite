# Rust quality baseline

Date: 2026-07-10

Updated: 2026-07-25

## Purpose

This baseline defines Ferrite's enforceable Rust API, safety, dependency,
documentation, release, and optimization standards. It uses primary project
sources and separates stable guidance from context-dependent lint opinions.

## Primary sources

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) for
  naming, traits, predictability, error design, metadata, and documentation.
- [Rust release notes](https://doc.rust-lang.org/stable/releases.html) for the
  current stable compiler and compatibility changes.
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
- [Cargo SemVer guidance](https://doc.rust-lang.org/cargo/reference/semver.html)
  for source, feature, platform, and MSRV compatibility review.
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
- [Rust Performance Book](https://nnethercote.github.io/perf-book/) and
  [Criterion.rs](https://bheisler.github.io/criterion.rs/book/index.html) for
  profiling, allocation analysis, and statistical microbenchmarks.
- [Rust Fuzz Book](https://rust-fuzz.github.io/book/),
  [Miri](https://github.com/rust-lang/miri/), the
  [rustc coverage guide](https://doc.rust-lang.org/rustc/instrument-coverage.html),
  and the [sanitizer guide](https://doc.rust-lang.org/beta/unstable-book/compiler-flags/sanitizer.html)
  for dynamic and coverage-guided validation.
- [RustSec](https://rustsec.org/) and
  [`cargo-deny`](https://embarkstudios.github.io/cargo-deny/) for advisory,
  license, source, and dependency graph policy.

## Requirement classification

- **Rust or Cargo requirement:** behavior required by the language, compiler,
  package format, or registry. Violating it is incorrect or prevents the
  supported tool workflow.
- **Ferrite requirement:** a repository contract accepted in an ADR or a
  maintained policy. Changes must preserve it unless a replacement decision is
  accepted.
- **Recommendation:** current primary-source guidance or a review finding that
  should be evaluated against Ferrite's measured needs. It is not a language
  rule and must not be applied mechanically.

The Rust API Guidelines, Clippy's opt-in groups, profiling techniques, PGO,
Miri, sanitizers, fuzzing, and Criterion are recommendations. Unsafe operation
preconditions, architecture and target-feature validity, Cargo manifest
requirements, and registry publication rules are normative.

## Ferrite repository requirements

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
13. Prefer internal in-place helpers when an inference stage has exclusive
    ownership of an existing buffer. Preserve allocating public APIs, compare
    against an independent arithmetic oracle, and require real-model token
    parity before accepting the allocation reduction.

## 2026-07-25 gap review

This review inspected the workspace manifests, crate roots, CI and release
workflows, maintained documentation, repository checks, public rustdoc, unsafe
call sites, and performance evidence. `scripts/check_docs.py` and
`scripts/check_repo.py` passed. Ferrite already has a clear public documentation
route, a performance golden path, a clean workspace layout, an MSRV lane,
multi-architecture tests, runtime SIMD dispatch, package inspection, SemVer
checks at release, dependency policy, SBOMs, and build attestations.

### Priority 0

- [x] **Ferrite requirement, define the server library boundary.**
  `ferrite-server` now states that its Rust library is not a stable published
  API, hides package-internal modules from generated documentation, and denies
  missing documentation on the intended surface. The direct
  `RUSTDOCFLAGS="-D warnings -D missing-docs" cargo doc -p ferrite-server
  --no-deps --locked` probe passes.
- [x] **Recommendation, add tested crate examples.** Each of the three
  publishable crates now has a small, copyable crate-level example, and all
  three pass the doctest gate. Continue adding examples at principal loading,
  tokenization, session, sampling, and cache entry points when those APIs are
  changed. [Rustdoc recommends examples](https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html)
  and the [API Guidelines](https://rust-lang.github.io/api-guidelines/checklist.html)
  include them in the public API checklist.
- [x] **Recommendation, test current stable separately from the pin.** Rust
  1.97.1 is current as of this review, while development and ordinary CI use
  1.96.1 and the MSRV lane uses 1.96.0. Preserve the reproducible pin and MSRV
  check. CI now adds a current-stable, all-feature check so new compiler
  warnings and downstream compatibility are found before a release. See the
  [current Rust release notes](https://doc.rust-lang.org/stable/releases.html)
  and Cargo's [MSRV support expectations](https://doc.rust-lang.org/cargo/reference/rust-version.html#support-expectations).
- [ ] **Recommendation, fuzz untrusted parsers.** Add bounded `cargo fuzz`
  targets for GGUF metadata and tensor tables, tokenizer metadata and byte
  handling, OpenAI JSON request schemas, generated tool-call JSON, and SSE
  parsing. Seed the corpus from source-built fixtures, keep model binaries out
  of Git, retain minimized crashing inputs, and run a scheduled bounded job.
  Fuzz coverage must demonstrate that targets progress beyond shallow header
  rejection. The [Rust Fuzz Book](https://rust-fuzz.github.io/book/) documents
  target setup, CI, structure-aware inputs, and coverage.

### Priority 1

- [ ] **Recommendation, add dynamic safety lanes.** Run the portable,
  platform-independent parser, tokenizer, cache, and scalar tests under Miri.
  Run bounded AddressSanitizer checks for memory access defects and
  ThreadSanitizer checks for scheduler and cache races on supported targets.
  Keep architecture intrinsics, memory mapping, inline assembly, and other
  unsupported paths outside a Miri claim. Start these nightly-based jobs as
  scheduled evidence before considering them merge blockers. Miri's
  [documented scope](https://github.com/rust-lang/miri/) and rustc's
  [sanitizer guide](https://doc.rust-lang.org/beta/unstable-book/compiler-flags/sanitizer.html)
  define the limits of these claims.
- [ ] **Recommendation, publish diagnostic coverage.** Use rustc's
  instrumentation coverage to find untested error branches in GGUF parsing,
  request validation, cancellation, and bounded-resource handling. Do not set
  an arbitrary repository-wide percentage gate. Require direct tests for
  security, unsafe-wrapper, and previously regressed branches. Rustc documents
  function, instantiation, line, and region coverage in its
  [coverage guide](https://doc.rust-lang.org/rustc/instrument-coverage.html).
- [x] **Recommendation, make publish intent uniform.** Set
  `publish = ["crates-io"]` explicitly on `ferrite-model` and
  `ferrite-inference`, matching `ferrite-fixtures`, while retaining
  `publish = false` for the CLI and server. Cargo's default is valid, but an
  explicit allowlist makes the release boundary reviewable. See Cargo's
  [`publish` field](https://doc.rust-lang.org/cargo/reference/manifest.html#the-publish-field).
- [ ] **Recommendation, run public API compatibility before tagging.** Keep the
  existing release-time `cargo semver-checks` gate, and add a pull-request or
  manually dispatched check when a published crate's public surface changes.
  Static API checks do not replace review of runtime semantics, feature
  behavior, MSRV, or documented guarantees. Cargo's
  [SemVer chapter](https://doc.rust-lang.org/cargo/reference/semver.html)
  explicitly includes features, platform requirements, and MSRV changes.

### Priority 2

- [ ] **Recommendation, centralize repeated external requirements.** Move
  shared requirements such as `serde_json` and `sha2` into
  `[workspace.dependencies]` when touched. Cargo supports workspace dependency
  inheritance, which prevents accidental version-policy drift without
  introducing a new crate boundary. See
  [Cargo workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html).
- [ ] **Recommendation, split only mixed ownership.** Large files are not a
  defect by themselves. When changing `ferrite-server/src/runtime.rs`,
  `ferrite-server/src/config.rs`, or `ferrite-cli/src/args.rs`, separate
  parsing, policy, engine, and protocol concerns only where the resulting
  module has a smaller public contract and independent tests. Do not perform a
  line-count-only rewrite.

## Low-overhead optimization order

1. Profile a locked release build on a quiet host and rank CPU hot spots,
   allocation sites, cache misses, and scheduler waits. The Rust Performance
   Book recommends [profiling before modifying code](https://nnethercote.github.io/perf-book/profiling.html).
2. Continue the accepted allocation pattern: reuse exclusively owned scratch,
   reserve known capacity, and avoid hot clones only where allocation profiling
   identifies them. Require pointer or capacity evidence plus exact output
   parity. The Performance Book documents the cost and profiling of
   [heap allocations](https://nnethercote.github.io/perf-book/heap-allocations.html).
3. Prefer iterators, pre-sliced chunks, and explicit range assertions before
   unchecked indexing. The Performance Book documents these as safe ways to
   help LLVM remove bounds checks. Use `get_unchecked` only after assembly and
   measurement prove that a remaining check is material. See its
   [bounds-check guidance](https://nnethercote.github.io/perf-book/bounds-checks.html).
4. Keep the portable provider and runtime feature detection for distributed
   binaries. Rust's [`core::arch` contract](https://doc.rust-lang.org/stable/core/arch/)
   requires both correct target selection and CPU-feature support before
   entering an intrinsic.
5. Use Criterion only for stable, isolated kernels or helpers whose signal is
   hidden by end-to-end noise. It complements, and never replaces, Ferrite's
   real-model eval suite. Criterion describes itself as a
   [statistics-driven microbenchmark tool](https://bheisler.github.io/criterion.rs/book/index.html).
6. Evaluate PGO with representative prefill, decode, mixed-prompt, batching,
   and HTTP workloads. Accept it only when a non-PGO portable artifact remains
   available and the full parity, TTFT, throughput, memory, CPU, and tail
   latency gates pass. The [rustc PGO guide](https://doc.rust-lang.org/rustc/profile-guided-optimization.html)
   explains how representative runtime profiles inform code generation.
7. Keep CPU inference outside Tokio executor workers and avoid oversubscription
   between the async runtime and Rayon. Ferrite's bounded admission and
   dedicated inference pool already implement this direction, so any change
   needs contention and cancellation evidence. Tokio's
   [`spawn_blocking` guidance](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html)
   recommends explicit limits for CPU-bound blocking work.

No finite test, fuzzer run, sanitizer run, or evaluation can prove that a
regression is impossible. Release and optimization reports must state the exact
covered models, inputs, architectures, features, and resource gates, and must
not turn those observations into an absolute guarantee.

## Repository enforcement

The baseline is implemented through `rust-toolchain.toml`, workspace manifests,
crate-level lints, release profiles, `deny.toml`, GitHub Actions,
`scripts/check_docs.py`, `scripts/check_repo.py`, package checks, and the eval
harness. See [ADR 0012](../adr/0012-open-source-quality-baseline.md) for the
baseline, [ADR 0020](../adr/0020-rust-2024-and-bounded-test-harnesses.md) for
the current edition and test layout, and [evaluation](../evaluation.md) for
exact commands.
