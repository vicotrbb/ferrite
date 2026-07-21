# In-place RoPE allocation diagnostic

Date: 2026-07-21

## Scope

This diagnostic evaluates one bounded inference allocation change. Every
transformer layer previously replaced the query and key vectors while applying
rotary position encoding. The head-level helper also allocated a temporary
vector for every query and key head before copying it into each replacement.

The candidate rotates each independent coordinate pair inside the projection
vectors that already own those values. For a model with `L` layers, `Hq` query
heads, and `Hkv` key-value heads, one single-stream token step avoids
`L * (Hq + Hkv + 2)` RoPE-related vector allocations. A batched step avoids the
same number for every batch member. The public allocating `apply_rope` API is
unchanged.

## Correctness evidence

The focused unit test compares both supported layouts against an independent
copy of the previous allocating algorithm. It requires bit-identical F32
outputs, including coordinates outside the configured rotary range. It also
asserts that the in-place vector pointer and capacity remain unchanged.

The single-session and batched-session implementations both use the same
in-place helper. Existing scalar-reference, batched-decode, architecture, and
real-model gates cover those paths in the full validation run.

## Fixed diagnostic inputs

- Host: Apple M5 Pro, 15 logical CPUs
- OS: macOS 26.5.2 arm64, build 25F84
- Toolchain: Rust 1.96.0, LLVM 22.1.2
- Source base: commit `c807ed585f3154f889380ba9ffafd9357f82bf92`
  on `main`, with the candidate evaluated in a dirty working tree
- Cargo profile: repository `release` profile, no `RUSTFLAGS`
- Baseline CLI SHA-256:
  `ab158ca67378a5b73bfad2f4dc3083769b0b8c8525f7efebb5ea66e0eae9d88c`
- Candidate CLI SHA-256:
  `d70ebad90728aa939f19edf61fb5ef08efbc581ca28e7e710c68055acccf2229`
- Model: Qwen2.5 0.5B Instruct Q4_K_M, 491,400,032 bytes
- Model SHA-256:
  `74a4da8c9fdbcd15bd1f6d01d621410d31c6fc00986f5eb687824e7b93d7a9db`
- Prompt: `Write a short story about a rusty robot who learns to sail.`
- Measured decode steps: 512 per run
- Repetitions: three interleaved baseline and candidate runs
- Workers: ten, selected automatically
- Kernel provider: automatic

Before measurement, `scripts/eval_suite.py --preflight-only` rejected the host.
The one-minute load was 10.235, or 0.682 per logical CPU, above the accepted
0.250 threshold. RSS and CPU were not sampled. Timing is therefore diagnostic
only and cannot support a release throughput claim.

## Command

```sh
<baseline-or-candidate-binary> \
  --model target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --prompt 'Write a short story about a rusty robot who learns to sail.' \
  --benchmark-runs 512
```

The order was baseline, candidate, candidate, baseline, baseline, candidate.
A separate 128-step run from each binary retained the complete ordered token
trace for hashing.

## Result

| Variant | Decode step samples | Median decode step | Diagnostic rate | Token trace SHA-256 |
| --- | --- | ---: | ---: | --- |
| Baseline | 17.593, 18.304, 15.833 ms | 17.593 ms | 56.84 tok/s | `ff342fb6b38a5f301d61dab6af424615a33dce9cc75ff1e9c026a2b40aa3674a` |
| In-place RoPE | 16.100, 16.020, 18.042 ms | 16.100 ms | 62.11 tok/s | `ff342fb6b38a5f301d61dab6af424615a33dce9cc75ff1e9c026a2b40aa3674a` |

The contaminated-host median moved 8.49% in the favorable direction. The wide
sample range prevents treating that timing as accepted performance evidence.
The exact complete token-trace match is accepted correctness evidence.

## Acceptance

Accepted as a deterministic allocation reduction with an unchanged public API,
independent bit-parity coverage, and exact real-model trace parity. No latency,
throughput, CPU, RSS, or tail-latency claim is promoted from this run. Those
claims require the full repeated eval on a host that passes preflight.
