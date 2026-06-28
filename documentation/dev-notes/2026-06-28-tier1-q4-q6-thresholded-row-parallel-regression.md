# 2026-06-28 Tier 1 Q4_K/Q6_K Thresholded Row Parallel Regression

## Scope

This note records a failed Tier 1 scheduling experiment for Qwen2.5-1.5B. The
experiment attempted to keep Q4_K and Q6_K aarch64 NEON row-parallel scheduling
only for large row-work matrices, while routing smaller matrices through a
serial NEON row loop to avoid Rayon overhead.

The experiment was not retained.

## Motivation

Fresh Qwen2.5-1.5B benchmark-token profiling on commit `1ff061a` showed the
model remained below the Tier 1 throughput target:

```text
benchmark_runs=3
benchmark_total_ns=836914500
benchmark_avg_ns=278971500
profile_benchmark_token_total_ns=104289919
profile_benchmark_token_role=ffn_down:Q4_K:1536:8960:7741440:10697419
profile_benchmark_token_role=ffn_down:Q6_K:1536:8960:11289600:13590793
profile_benchmark_token_role=ffn_gate:Q4_K:8960:1536:7741440:22037708
profile_benchmark_token_role=ffn_up:Q4_K:8960:1536:7741440:22895997
profile_benchmark_token_role=output:Q6_K:151936:1536:191439360:15143000
```

The two-thread baseline was slower:

```text
benchmark_runs=3
benchmark_total_ns=1220022333
benchmark_avg_ns=406674111
profile_benchmark_token_total_ns=221292783
profile_benchmark_token_role=ffn_down:Q4_K:1536:8960:7741440:24833918
profile_benchmark_token_role=ffn_down:Q6_K:1536:8960:11289600:32719708
profile_benchmark_token_role=ffn_gate:Q4_K:8960:1536:7741440:48083998
profile_benchmark_token_role=ffn_up:Q4_K:8960:1536:7741440:48365914
profile_benchmark_token_role=output:Q6_K:151936:1536:191439360:45509667
```

## Experiment

The temporary implementation added an aarch64-only `Aarch64NeonRowParallel`
backend marker for Q4_K and Q6_K, then used row-parallel NEON only when:

```text
rows * (cols / 256) >= 16384
```

The initial red check required the new backend markers:

```text
error[E0599]: no variant, associated function, or constant named `Aarch64NeonRowParallel` found for enum `Q4KMatVecBackend`
error[E0599]: no variant, associated function, or constant named `Aarch64NeonRowParallel` found for enum `Q6KMatVecBackend`
```

After implementation and fixture correction, the focused scheduling tests
passed:

```text
cargo test -p ferrite-inference large_row_work_uses_neon_row_parallel -- --nocapture
test scalar::quantized_tests::q4_k_large_row_work_uses_neon_row_parallel ... ok
test scalar::quantized_tests::q6_k_large_row_work_uses_neon_row_parallel ... ok
```

The full quantized group also passed:

```text
cargo test -p ferrite-inference quantized_tests -- --nocapture
test result: ok. 18 passed; 0 failed
```

## Regression Evidence

After rebuilding release, the default-pool Qwen2.5-1.5B benchmark regressed:

```text
cargo build --release -p ferrite-cli
/usr/bin/time -l target/release/ferrite --model target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf --prompt 'hello world' --benchmark-runs 3 --profile-benchmark-token
```

Output:

```text
benchmark_runs=3
benchmark_total_ns=1073690959
benchmark_avg_ns=357896986
profile_benchmark_token_total_ns=222349799
profile_benchmark_token_role=ffn_down:Q4_K:1536:8960:7741440:23544375
profile_benchmark_token_role=ffn_down:Q6_K:1536:8960:11289600:29703377
profile_benchmark_token_role=ffn_gate:Q4_K:8960:1536:7741440:51001127
profile_benchmark_token_role=ffn_up:Q4_K:8960:1536:7741440:49749332
profile_benchmark_token_role=output:Q6_K:151936:1536:191439360:24722958
        5.14 real         4.60 user         2.73 sys
          2091089920  maximum resident set size
          2268075456  peak memory footprint
```

Compared with the fresh default-pool baseline, the benchmark average regressed
from `278971500` ns to `357896986` ns, and the profiled token total regressed
from `104289919` ns to `222349799` ns.

## Result

The thresholded Q4_K/Q6_K row-parallel scheduling experiment was reverted before
commit. The retained implementation keeps the existing row-parallel Q4_K and
Q6_K NEON paths.

Do not repeat this threshold shape without a different hypothesis and a fresh
benchmark gate. The regression indicates that serializing the smaller Q4_K/Q6_K
projection matrices costs more than it saves on the local Apple M1 Pro
Qwen2.5-1.5B profile.
