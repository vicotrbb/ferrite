# 2026-06-27 Tier 1 SmolLM2 1.7B Q6_K Fused NEON Dot

## Scope

This benchmark records the incremental effect of replacing the aarch64 Q6_K
NEON path's decode-then-dot block loop with a fused block-dot helper after the
Q4_K fused NEON slice.

This is still not a Tier 1 throughput pass. The local 2-thread run remains
below the `>= 10 tok/s` target.

## Tree State

- Branch: `main`
- Implementation commit: `0f194f2`
- Working tree before note: clean

## Hardware and OS

- Machine: Apple M1 Pro
- Logical CPUs: 8
- RAM: 17,179,869,184 bytes
- OS: macOS Darwin 23.5.0 arm64

## Model

- Repo: `bartowski/SmolLM2-1.7B-Instruct-GGUF`
- File: `SmolLM2-1.7B-Instruct-Q4_K_M.gguf`
- Local path: `target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf`
- Local size reported by Ferrite: 1,055,609,824 bytes
- Scalar weight bytes reported by Ferrite: 1,053,827,072 bytes
- Quantization: Q4_K_M GGUF mixture

## Verification

Red test before implementation:

```text
error[E0432]: unresolved import `super::neon_q6_k_block_dot`
```

Focused green test:

```sh
cargo test -p ferrite-inference q6_k -- --nocapture
```

Full verification before the implementation commit:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p ferrite-inference --target x86_64-unknown-linux-gnu --tests
git diff --check
```

All commands passed.

## Results

| Run | benchmark_avg_ns | Approx Tok/s | Real Time | User Time | Max RSS | Peak Footprint |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Q4_K fused NEON dot, default Rayon pool | 229,523,316 | 4.357 | 3.81 s | 4.47 s | 1,522,860,032 bytes | 2,123,994,432 bytes |
| Q4_K + Q6_K fused NEON dot, default Rayon pool | 224,075,783 | 4.463 | 3.28 s | 3.99 s | 1,475,854,336 bytes | 2,123,912,448 bytes |
| Q4_K fused NEON dot, `RAYON_NUM_THREADS=2` | 339,890,275 | 2.942 | 3.88 s | 3.85 s | 1,475,592,192 bytes | 2,123,633,856 bytes |
| Q4_K + Q6_K fused NEON dot, `RAYON_NUM_THREADS=2` | 331,853,141 | 3.013 | 3.70 s | 3.47 s | 1,473,363,968 bytes | 2,123,846,848 bytes |

The prior Q4_K fused NEON rows are from
`documentation/benchmarks/2026-06-27-tier1-smollm2-1-7b-q4k-fused-neon.md`.

## Evidence

Parity check:

```text
generated_token_ids=18,198,3725,198,198,788
generated_match=true
expected_token_id=18
match=true
```

Default Rayon-pool benchmark:

```text
benchmark_runs=5
benchmark_total_ns=1120378917
benchmark_avg_ns=224075783
        3.28 real         3.99 user         1.92 sys
          1475854336  maximum resident set size
          2123912448  peak memory footprint
```

Two-thread benchmark:

```text
benchmark_runs=5
benchmark_total_ns=1659265708
benchmark_avg_ns=331853141
        3.70 real         3.47 user         0.95 sys
          1473363968  maximum resident set size
          2123846848  peak memory footprint
```

Profile summary after the change:

```text
profile_next_token_total_ns=116119081
profile_next_token_role=ffn_down:Q6_K:2048:8192:13762560:16103001
profile_next_token_role=v_proj:Q6_K:2048:2048:3440640:4715250
profile_next_token_role=output:Q6_K:49152:2048:82575360:10366125
```

## Interpretation

The fused Q6_K NEON block dot produced a small retained improvement on this
model after the larger Q4_K fused-dot gain: the default-pool average improved
from about 0.230 seconds to about 0.224 seconds, and the 2-thread average
improved from about 0.340 seconds to about 0.332 seconds.

The profile summary shows Q6_K `ffn_down` improving from the previous roughly
21 ms run to about 16 ms here, while the large Q6_K output projection remains a
visible hot path.

Ferrite remains below the Tier 1 throughput target.

