# ADR 0014: Portable kernel provider and thread topology

Date: 2026-07-13

Status: Accepted

## Context

Ferrite already had scalar, Arm NEON, Arm DotProd and I8MM, and x86 AVX2
kernels. Each optimized entry was guarded, but feature probes were repeated
inside matrix implementations. There was no request-level way to force the
portable oracle, no stable capability diagnostic, and batch admission did not
explicitly reject sessions with different execution policies.

A diagnostic next-token profile on the Qwen2.5 0.5B Q4_K_M reference model
ranked the quantized feed-forward projections as the largest recorded matvec
group and the vocabulary projection as the next individual hotspot. The host
was busy, so that profile is useful only for hypothesis ordering. It is not a
performance result and does not justify a new ISA kernel or dependency.

## Decision

Ferrite has two built-in kernel providers:

- `auto` uses a proven optimized implementation only after the central runtime
  capability detector reports its required feature.
- `portable` disables every architecture-specific implementation and uses the
  safe reference path.

A provider can disable a feature, but it cannot force an unavailable feature.
The provider is part of `ScalarExecutionOptions`, the CLI and server expose
`--kernel-provider`, and prefix-cache compatibility fingerprints include it.
Sessions in one execution batch must have identical options.

The capability boundary reports current NEON, DotProd, I8MM, SVE2, AVX2,
AVX-VNNI, and AVX512-VNNI support. Only NEON, DotProd, I8MM, and AVX2 currently
select Ferrite kernels. The remaining values are diagnostic inputs for future
experiments, not claims of implemented acceleration.

Automatic thread selection uses the largest macOS performance level. On Linux
it respects the process CPU allow-list, prefers the highest reported CPU
capacity or core type on heterogeneous systems, and otherwise chooses unique
physical cores before falling back to available parallelism. Explicit CLI and
environment overrides remain exact. The HTTP server leaves one recommended CPU
slot for service work when the normal provider would otherwise occupy the full
automatic count. A memory-bound policy already leaves capacity and receives no
additional reduction.

CI is configured for native Linux x86_64, Linux arm64, macOS arm64, macOS
x86_64, and Windows x86_64. A focused macOS arm64 job builds and runs the x86_64
provider test through Rosetta.

## Consequences

Feature detection no longer leaks into model execution code. Tests can compare
the optimized path with a provider that is guaranteed not to enter target
feature functions. Operators also receive the selected provider and detected
feature list at process startup.

The portable parity gate exposed a Q6_K argmax fallback that used a zero-sized
row chunk when one valid quantization block spanned multiple small rows. The
fallback now computes the full scalar matvec for non-row-aligned layouts before
selecting argmax, with a focused regression test.

No throughput improvement is claimed. Thread defaults and the provider boundary
are reliability and portability changes. Clean repeated eval artifacts are
required before retaining any performance-sensitive change derived from them.

NUMA-aware allocation is not implemented. Ferrite has no clean multi-socket
end-to-end evidence yet, and binding the shared model mapping or per-session KV
state speculatively could regress common single-node hosts. The Linux topology
probe creates a bounded place to add NUMA evidence later without changing model
semantics.

## Alternatives Considered

- Force target features with build-wide `target-cpu=native`. Rejected because
  the resulting binary may execute unsupported instructions on another CPU and
  prior machine-specific evidence did not show a reliable end-to-end gain.
- Add KleidiAI immediately. Rejected for now because it adds a C and assembly
  supply-chain boundary, packing work, and architecture-specific maintenance
  without a clean matching-hardware evaluation.
- Add oneDNN immediately. Rejected for now because Ferrite's decode workload is
  dominated by GGML quantized matvec formats, and no comparable end-to-end
  result demonstrates that conversion and integration overhead pays back.
- Add SVE2, SME2, VNNI, AVX512-VNNI, or AMX dispatch without a tested kernel.
  Rejected because capability detection alone is not an optimization and must
  never imply execution support.
- Bind memory to NUMA nodes based on topology alone. Rejected until a native
  multi-node gate measures throughput, TTFT, and remote-memory effects.

## Evidence

- `crates/ferrite-inference/src/scalar/kernels.rs` owns capability detection and
  provider policy.
- `crates/ferrite-inference/tests/batched_decode.rs` covers provider token
  parity and mixed-provider batch rejection.
- `crates/ferrite-inference/src/threading.rs` owns macOS and Linux topology
  selection plus service-aware defaults.
- `.github/workflows/ci.yml` declares the native and Rosetta platform matrix.
- The Rust standard library documents the required runtime feature guard for
  target-feature functions: <https://doc.rust-lang.org/stable/core/arch/>.
- Arm's KleidiAI repository describes its micro-kernel, ISA, build, and release
  integration surface: <https://github.com/ARM-software/kleidiai>.
- oneDNN's primary repository describes its supported runtime and Apache-2.0
  distribution: <https://github.com/uxlfoundation/oneDNN>.
- GitHub's runner reference identifies the architectures behind the CI labels:
  <https://docs.github.com/en/actions/reference/runners/github-hosted-runners>.
