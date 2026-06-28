# 2026-06-27 Tier 1 SmolLM2 1.7B Token-ID Benchmark Path

## Scope

This benchmark records the effect of changing `--benchmark-runs` to use a
token-id-only session path after the initial prompt token. The repeated
benchmark loop now computes the next token ID without returning a full logits
vector for every repeated token.

This is benchmark-path evidence, not a general logits API removal. Normal
`accept_token`, top-logits, generation, and profiled paths still preserve logits
where their callers need them.

This is still not a Tier 1 throughput pass. The local 2-thread run remains
below the `>= 10 tok/s` target.

## Tree State

- Branch: `main`
- Implementation commit: `1f2d69d`
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

Red tests before implementation:

```text
error[E0432]: unresolved import `super::q6_k::q6_k_argmax_mul_vec`
error[E0599]: no method named `accept_token_id` found for struct `ScalarLlamaSession<'a>`
```

Focused green tests:

```sh
cargo test -p ferrite-inference q6_k_argmax_mul_vec_matches_full_matvec_argmax -- --nocapture
cargo test -p ferrite-inference scalar_session_accepts_token_id_without_returning_logits -- --nocapture
cargo test -p ferrite-cli cli_benchmarks_repeated_next_token_runs_after_loading_once -- --nocapture
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
| Q4_K + Q6_K fused NEON dot, default Rayon pool | 224,075,783 | 4.463 | 3.28 s | 3.99 s | 1,475,854,336 bytes | 2,123,912,448 bytes |
| Token-id benchmark path, default Rayon pool | 181,434,575 | 5.512 | 2.73 s | 4.17 s | 1,474,101,248 bytes | 2,123,633,856 bytes |
| Q4_K + Q6_K fused NEON dot, `RAYON_NUM_THREADS=2` | 331,853,141 | 3.013 | 3.70 s | 3.47 s | 1,473,363,968 bytes | 2,123,846,848 bytes |
| Token-id benchmark path, `RAYON_NUM_THREADS=2` | 297,401,758 | 3.363 | 3.27 s | 3.37 s | 1,474,543,616 bytes | 2,123,650,368 bytes |

The prior fused NEON rows are from
`documentation/benchmarks/2026-06-27-tier1-smollm2-1-7b-q6k-fused-neon.md`.

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
benchmark_total_ns=907172875
benchmark_avg_ns=181434575
        2.73 real         4.17 user         2.18 sys
          1474101248  maximum resident set size
          2123633856  peak memory footprint
```

Two-thread benchmark:

```text
benchmark_runs=5
benchmark_total_ns=1487008792
benchmark_avg_ns=297401758
        3.27 real         3.37 user         0.83 sys
          1474543616  maximum resident set size
          2123650368  peak memory footprint
```

## Interpretation

The token-id-only benchmark path improves the local default-pool repeated-token
average from about 0.224 seconds to about 0.181 seconds. The 2-thread average
improves from about 0.332 seconds to about 0.297 seconds.

This narrows benchmark semantics toward token generation throughput: repeated
benchmark tokens no longer pay to return logits that the CLI benchmark does not
print or inspect. The model still computes the output projection argmax, and the
normal logits-returning APIs remain available.

Ferrite remains below the Tier 1 throughput target.

