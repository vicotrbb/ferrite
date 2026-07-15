# ADR 0018: Retained matrix storage and bounded row decoding

Date: 2026-07-14

Status: Accepted

## Context

Ferrite retained supported quantized tensors as validated ranges of one
read-only GGUF mapping, but two dense and row-access paths still created
avoidable private allocations.

First, mapped F16 and BF16 matrices were decoded into owned F32 matrices during
loading. Second, token embedding lookup for Q4_K, Q5_K, and Q6_K decoded the
entire matrix before copying one row. The second path is especially costly for
large vocabularies. It can materialize 402,653,184 bytes for the
49,152 by 2,048 SmolLM2 embedding and 394,002,432 bytes for the
32,064 by 3,072 Phi-3 embedding even though one lookup needs only one row.

The macOS physical-footprint soak gate exposed this as a request-shaped private
allocation. A pre-fix Phi-3 diagnostic failed with physical-footprint tail
ranges of 397,771,040 bytes on the default route and 394,625,312 bytes on the
continuous-batched route. Raising the soak tolerance would have hidden an
allocation defect instead of fixing it.

## Decision

Ferrite retains F16 and BF16 matrix bytes in the shared mapped model file and
converts lanes while accumulating a matvec. The portable implementation defines
the result. Runtime-gated Arm NEON and x86 AVX2 plus F16C implementations use
the same accumulation shape, with safe wrappers validating dimensions, finite
values, and CPU capabilities before entering unsafe SIMD code.

Q4_K, Q5_K, and Q6_K row access validates the complete matrix storage but
decodes only the minimal quantization-block window that intersects the selected
row. It then returns the row slice from that bounded window. This also supports
the existing Q4_K and Q6_K case where one 256-value block spans row boundaries.

Owned and mapped byte-backed matrices share one internal storage abstraction.
F32 matrices remain owned. Full vocabulary logits remain available when a
sampling or grammar policy requires them.

Neither change may alter deterministic token IDs. Real-model trace comparison,
portable-provider comparison, native tests, Linux x86 cross-target Clippy, and
the server soak remain acceptance gates.

## Consequences

Model loading no longer expands F16 or BF16 matrices to F32. Token lookup no
longer allocates an F32 copy of a complete K-quantized embedding matrix. The
largest temporary row decode is bounded by the selected row plus at most two
partial boundary blocks.

Direct dense-16 conversion can trade conversion work for lower memory traffic.
No universal throughput improvement follows from this decision. Performance
claims still require comparable clean repeated artifacts.

The unsafe boundary grows by two small architecture-specific dense-16 modules.
Their public entry remains safe, runtime gated, and covered by exact-output
tests. The quantized row-window change uses safe Rust only.

## Alternatives Considered

- Cache a fully decoded embedding matrix. Rejected because it turns a transient
  defect into a large retained allocation.
- Require every K-quantized row to align to a block. Rejected because Ferrite
  already supports validated layouts where a block spans several small rows.
- Decode the whole matrix and rely on allocator purging. Rejected because purge
  timing is not a memory bound and produced unstable soak results.
- Gate only on total RSS. Rejected on macOS because clean mapped model pages can
  enter or leave RSS independently of private request allocations.
- Increase the 16 MiB soak tolerance. Rejected because the observed allocation
  was about 376 to 379 MiB and had a source-level bounded alternative.

## Evidence

- `scalar::matrix::rows::tests` proves that Q4_K and Q6_K rows crossing block
  boundaries retain exact values and that the decoder receives only the
  intersecting block window.
- `scalar::dense16::tests` proves exact F16 and BF16 matvec agreement with the
  F32 accumulation path for portable and detected providers.
- `cargo test --locked -p ferrite-inference --features locus-kv` passed after
  the changes.
- Strict native and `x86_64-unknown-linux-gnu` Clippy passed with all targets
  and all features.
- The rejected pre-fix diagnostic is
  [`2026-07-14-122938`](../../scripts/evals/2026-07-14-122938-smollm2-1.7b-instruct-q4_k_m-multi.md).
- The accepted full diagnostic is
  [`2026-07-14-130458`](../../scripts/evals/2026-07-14-130458-smollm2-1.7b-instruct-q4_k_m-multi.md).
  Its four default and batched model routes retained exact token traces. The
  physical-footprint tail ranges were 1,196,032 and 3,080,192 bytes for
  SmolLM2, then 835,584 and 3,784,704 bytes for Phi-3, all below 16 MiB.
- [The bounded embedding-row diagnostic](../benchmarks/2026-07-14-bounded-embedding-row-decode.md)
  records the method, before-and-after evidence, and non-performance scope.
