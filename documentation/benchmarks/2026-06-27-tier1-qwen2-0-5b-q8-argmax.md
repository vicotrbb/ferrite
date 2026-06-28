# 2026-06-27 Tier 1 Qwen2 0.5B Q8_0 Argmax

## Scope

This benchmark records the incremental effect of routing Q8_0 output matrices
through a direct argmax matvec path. Before this slice, token-id-only generation
had a specialized argmax path for Q6_K, but Q8_0 output matrices still fell back
to full `mul_vec` output allocation before selecting the best token.

This is a local Qwen2.5-0.5B throughput improvement. It is not a full Tier 1
throughput pass because the 1.5B Qwen2 profile and broader Tier 1 coverage are
still unproven.

## Tree State

- Branch: `main`
- Implementation commit: this atomic Q8_0 argmax commit
- Working tree before slice: clean after `c4a813e`

## Hardware and OS

- Machine: Apple M1 Pro
- Logical CPUs: 8
- RAM: 17,179,869,184 bytes
- OS: macOS Darwin 23.5.0 arm64

## Model

- Repo: `bartowski/Qwen2.5-0.5B-Instruct-GGUF`
- File: `Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- Local path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- Local size reported by Ferrite: 397,808,192 bytes
- Scalar weight bytes reported by Ferrite: 391,859,712 bytes
- Output projection storage: Q8_0

## TDD and Verification

Red test before implementation:

```text
error[E0432]: unresolved import `super::q8_0::q8_0_argmax_mul_vec`
```

Focused green tests:

```sh
cargo test -p ferrite-inference q8_0_argmax_mul_vec_matches_full_matvec_argmax -- --nocapture
cargo test -p ferrite-inference q8_0 -- --nocapture
```

Release build before model checks:

```sh
cargo build --release -p ferrite-cli
```

## Commands

Parity check:

```sh
target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --prompt 'hello world' --generate-tokens 3 --expect-generated-token-ids 198,9707,11
```

Default Rayon-pool benchmark:

```sh
/usr/bin/time -l target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --prompt 'hello world' --benchmark-runs 5
```

Two-thread benchmark:

```sh
/usr/bin/time -l env RAYON_NUM_THREADS=2 target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --prompt 'hello world' --benchmark-runs 5
```

## Results

| Run | benchmark_avg_ns | Approx Tok/s | Real Time | User Time | Max RSS | Peak Footprint |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Q5_0 fused NEON dot, default Rayon pool | 184,524,158 | 5.42 | 2.47 s | 1.53 s | 525,418,496 bytes | 828,115,968 bytes |
| Q8_0 direct argmax, default Rayon pool | 93,986,125 | 10.64 | 1.03 s | 0.95 s | 827,703,296 bytes | 828,083,264 bytes |
| Q5_0 fused NEON dot, `RAYON_NUM_THREADS=2` | 190,746,900 | 5.24 | 2.24 s | 1.46 s | 782,401,536 bytes | 828,345,472 bytes |
| Q8_0 direct argmax, `RAYON_NUM_THREADS=2` | 92,898,566 | 10.76 | 1.14 s | 0.84 s | 685,539,328 bytes | 827,771,840 bytes |

The before rows are from
`documentation/benchmarks/2026-06-27-tier1-qwen2-0-5b-q5-neon-block-dot.md`.

## Evidence

Parity check:

```text
generated_token_ids=198,9707,11
generated_match=true
model_file_bytes=397808192
scalar_weight_bytes=391859712
kv_cache_bytes=122880
```

Default Rayon-pool benchmark:

```text
benchmark_runs=5
benchmark_cached_tokens=7
benchmark_total_ns=469930625
benchmark_avg_ns=93986125
model_file_bytes=397808192
scalar_weight_bytes=391859712
kv_cache_bytes=172032
        1.03 real         0.95 user         0.25 sys
           827703296  maximum resident set size
           828083264  peak memory footprint
```

Two-thread benchmark:

```text
benchmark_runs=5
benchmark_cached_tokens=7
benchmark_total_ns=464492833
benchmark_avg_ns=92898566
model_file_bytes=397808192
scalar_weight_bytes=391859712
kv_cache_bytes=172032
        1.14 real         0.84 user         0.20 sys
           685539328  maximum resident set size
           827771840  peak memory footprint
```

## Interpretation

Direct Q8_0 argmax avoids materializing the full logits vector for Q8_0 output
matrices in token-id-only generation. On the local Qwen2.5-0.5B profile, that
improved the default-pool cached-token average from about 0.185 seconds to about
0.094 seconds. The two-thread average improved from about 0.191 seconds to about
0.093 seconds.

This is the first retained local Qwen2.5-0.5B Q4_K_M benchmark above 10 tok/s,
including the two-thread run. Ferrite still needs equivalent evidence for
larger Tier 1 models and broader prompts before the Tier 1 throughput target can
be marked complete.
