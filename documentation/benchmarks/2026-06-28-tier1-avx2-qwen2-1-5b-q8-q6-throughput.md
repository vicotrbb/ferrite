# 2026-06-28 Tier 1 AVX2 Qwen2.5 1.5B Q8_0 and Q6_K Throughput

## Scope

This benchmark records bounded x86_64 AVX2 throughput for the
Qwen2.5-1.5B-Instruct Q8_0 and Q6_K artifacts after their local and x86_64
six-prompt parity checks.

This is benchmark evidence only. It does not prove the Tier 1 throughput gate:
both quantizations remain below the 10 tok/s target on this bounded amd64 pod.

## Tree State

- Branch: `main`
- Commit before run: `758f635`
- Working tree before run: clean

## Environment

Kubernetes context:

```text
staging
```

Bounded pod:

```text
name: ferrite-avx2-qwen15-q8q6-throughput
image: rust:1.96-bookworm
cpu request: 500m
cpu limit: 2
memory request: 1Gi
memory limit: 8Gi
ephemeral-storage request: 10Gi
ephemeral-storage limit: 16Gi
node selector: kubernetes.io/arch=amd64
node: homelab-01
```

The runtime host reported:

```text
x86_64
```

`/proc/cpuinfo` included both `avx` and `avx2`.

Toolchain:

```text
cargo 1.96.0 (30a34c682 2026-05-25)
rustc 1.96.0 (ac68faa20 2026-05-25)
host: x86_64-unknown-linux-gnu
LLVM version: 22.1.2
```

The pod image had Rust installed in `/usr/local/cargo/bin`, but the default pod
`PATH` omitted that directory. The build and environment checks therefore used
`export PATH=/usr/local/cargo/bin:$PATH`.

The pod built the copied source release CLI:

```text
Finished `release` profile [optimized] target(s) in 9.17s
```

Pod workspace size after source, models, and build:

```text
3.2G /work/ferrite
```

The pod cgroup reported:

```text
memory.peak=6562594816
memory.max=8589934592
```

The peak is pod-wide evidence across source copy, release build, and benchmark
runs. The image did not include `/usr/bin/time`, so per-command maximum RSS was
not available without changing the pod package set.

## Artifacts

The pod-side SHA256 hashes matched the local artifacts:

```text
d7efb072e7724d25048a4fda0a3e10b04bdef5d06b1403a1c93bd9f1240a63c8  target/models/qwen2.5-1.5b-instruct-q8_0.gguf
e16d94f3b1eb243f6f6be9eee51090ef5dfd741324394fd5b6e0e425c33df5c7  target/models/qwen2.5-1.5b-instruct-q6_k.gguf
```

Ferrite reported:

```text
q8_0 model_file_bytes=1894532128
q8_0 scalar_weight_bytes=1888581632
q6_k model_file_bytes=1464178720
q6_k scalar_weight_bytes=1458228224
kv_cache_bytes=401408
```

## Protocol

- Host: bounded x86_64 AVX2 Kubernetes pod
- Prompt: `hello world`
- Prompt token IDs: `14990,1879`
- Benchmark runs: 5 repeated token-id decode steps after initial prompt
  next-token computation
- Execution policy: default only; Q8_K activation matvec disabled

Commands:

```sh
target/release/ferrite \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --prompt 'hello world' \
  --benchmark-runs 5

RAYON_NUM_THREADS=2 target/release/ferrite \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --prompt 'hello world' \
  --benchmark-runs 5

target/release/ferrite \
  --model target/models/qwen2.5-1.5b-instruct-q6_k.gguf \
  --prompt 'hello world' \
  --benchmark-runs 5

RAYON_NUM_THREADS=2 target/release/ferrite \
  --model target/models/qwen2.5-1.5b-instruct-q6_k.gguf \
  --prompt 'hello world' \
  --benchmark-runs 5
```

All four runs produced:

```text
next_token_id=198
benchmark_cached_tokens=7
benchmark_token_ids=9707,11,1879,0,2585
experimental_q8_k_activation_matvec=false
compare_q8_k_activation_matvec=false
q8_k_activation_matvec_policy=default_only
```

## Results

| Model | Thread setting | benchmark_avg_ns | Approx tok/s | Real time | User time | Sys time |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Q8_0 | default pool | 234,260,802 | 4.27 | 4.51 s | 1.93 s | 2.55 s |
| Q8_0 | `RAYON_NUM_THREADS=2` | 229,229,300 | 4.36 | 4.75 s | 1.86 s | 2.79 s |
| Q6_K | default pool | 1,356,563,424 | 0.74 | 13.35 s | 9.69 s | 5.51 s |
| Q6_K | `RAYON_NUM_THREADS=2` | 1,348,577,646 | 0.74 | 11.05 s | 9.66 s | 4.71 s |

## Interpretation

Q8_0 is the faster Qwen2.5-1.5B quantization on this bounded x86_64 AVX2 pod,
but it remains below the Tier 1 target. The best run measured about 229.2 ms
per benchmark token, or about 4.36 tok/s, with `RAYON_NUM_THREADS=2`.

Q6_K is much slower on this pod: about 0.74 tok/s for both default-pool and
two-thread runs. This is worse than the local aarch64 Q6_K measurement and
keeps Q6_K out of the current Tier 1 throughput path for Qwen2.5-1.5B.

The next x86_64 optimization slice should profile Q6_K and Q8_0 hot roles on
the amd64 pod before changing kernels. That follow-up profile is recorded in
`documentation/benchmarks/2026-06-28-tier1-avx2-qwen2-1-5b-q8-q6-profile.md`.
In particular, this run does not justify promoting the experimental Q8_K
activation matvec path or changing default dispatch policy.

## Cleanup

The pod was deleted after the checks. A final cleanup check:

```sh
kubectl get pod ferrite-avx2-qwen15-q8q6-throughput --context staging --ignore-not-found
```

returned no output.
