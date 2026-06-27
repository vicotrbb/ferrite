# Tier 1 Gate Status

Date: 2026-06-27

## Scope

Tier 1 covers 0.5B-1.7B models and is intended to prove:

- real matrix sizes;
- SIMD correctness;
- GQA variants;
- RoPE head dimensions used by larger Llama-family models;
- KV-cache growth and shrink behavior; and
- benchmark evidence before throughput claims.

## Current Verdict

Tier 1 is in progress. Ferrite now has several Tier 1 correctness harnesses and
the first scoped SIMD kernel on the local aarch64 host, but Tier 1 is not
complete.

The current implementation proves scalar GQA ratio coverage, scalar RoPE
coverage for `head_dim=64` and `head_dim=128`, session cache truncation, a
matvec reference-comparison gate, and aarch64 NEON F32, Q8_0, and Q5_0 matvec paths
checked against the scalar oracle. It does not yet prove AVX2 correctness,
Q4_K/Q6_K SIMD kernels, real 0.5B-1.7B model output, or Tier 1 throughput.

## Evidence Matrix

| Criterion | Status | Evidence |
| --- | --- | --- |
| GQA variants 1:1, 3:1, 4:1, 6:1, 7:1 | Proven for scalar attention harness | `documentation/dev-notes/2026-06-27-tier1-gqa-ratio-harness.md`; `cargo test -p ferrite-inference gqa_broadcasts_kv_heads_for_tier1_ratios -- --nocapture` |
| RoPE `head_dim=64` and `head_dim=128` | Proven for scalar RoPE harness | `documentation/dev-notes/2026-06-27-tier1-rope-head-dim-harness.md`; `cargo test -p ferrite-inference rope_rotates_full_tier1_head_dimensions -- --nocapture` |
| KV cache grows and shrinks across turns | Proven for scalar session cache | `documentation/dev-notes/2026-06-27-tier1-session-cache-truncation.md`; `cargo test -p ferrite-inference --test scalar_session_cache -- --nocapture` |
| Matvec kernels compare against scalar reference within explicit tolerance | Harness exists | `documentation/dev-notes/2026-06-27-tier1-matvec-kernel-check.md`; `cargo test -p ferrite-inference --test matvec_kernel_check -- --nocapture` |
| AArch64 SIMD correctness | Partially proven for F32, Q8_0, and Q5_0 matvec on local NEON host | `documentation/dev-notes/2026-06-27-tier1-aarch64-neon-f32-matvec.md`; `documentation/dev-notes/2026-06-27-tier1-aarch64-neon-q8-matvec.md`; `documentation/dev-notes/2026-06-27-tier1-aarch64-neon-q5-matvec.md`; targeted aarch64 backend tests |
| AVX2 correctness | Not proven | No x86_64 AVX2 kernel or x86 host evidence yet |
| Quantized SIMD correctness | Partially proven for Q8_0 and Q5_0 on local NEON host | `documentation/dev-notes/2026-06-27-tier1-aarch64-neon-q8-matvec.md`; `documentation/dev-notes/2026-06-27-tier1-aarch64-neon-q5-matvec.md`; Q4_K and Q6_K kernels are still scalar/direct paths |
| Real 0.5B-1.7B model output | Not proven | No Tier 1 model reference run recorded yet |
| Tier 1 throughput target | Not proven | No benchmark note proving `>= 10 tok/s` on 2 vCPU Q4_K_M |

## Fresh Full-Workspace Gate

Commands run after the aarch64 NEON Q5_0 matvec slice:

```sh
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

All commands passed.

## Remaining Work

- Add x86_64 AVX2 matvec behind ADR 0006's unsafe-boundary rules.
- Add reference-checked SIMD paths for the remaining quantized matvec kernels,
  starting with Q4_K or Q6_K.
- Run a Tier 1 model from `research/11-testing-model-registry.md` with a fixed
  reference profile and record parser, output, memory, and latency evidence.
- Benchmark Tier 1 decode throughput with hardware, model, quantization,
  prompt, thread count, and RSS details before making any throughput claim.
- Keep Tier 0's SmolLM2-360M CPU-only reference split documented as a caveat
  when comparing optimized CPU paths.
