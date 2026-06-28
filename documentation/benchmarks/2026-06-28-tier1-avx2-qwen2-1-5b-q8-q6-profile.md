# 2026-06-28 Tier 1 AVX2 Qwen2.5 1.5B Q8_0 and Q6_K Profile

## Scope

This benchmark records x86_64 AVX2 benchmark-token profile evidence for the
Qwen2.5-1.5B-Instruct Q8_0 and Q6_K artifacts after the bounded x86_64
throughput run showed Q8_0 at about 4.36 tok/s and Q6_K at about 0.74 tok/s.

This is profile evidence only. It does not change default dispatch policy and
does not prove the Tier 1 throughput gate.

## Tree State

- Branch: `main`
- Commit before run: `92b9be1`
- Working tree before run: clean

## Environment

Kubernetes context:

```text
staging
```

Bounded pod:

```text
name: ferrite-avx2-qwen15-q8q6-profile
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
Finished `release` profile [optimized] target(s) in 9.38s
```

Pod workspace size after source, models, and build:

```text
3.2G /work/ferrite
```

The pod cgroup reported:

```text
memory.peak=5933760512
memory.max=8589934592
```

The peak is pod-wide evidence across source copy, release build, and profile
runs.

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
kv_cache_bytes=172032
```

## Protocol

- Host: bounded x86_64 AVX2 Kubernetes pod
- Prompt: `hello world`
- Prompt token IDs: `14990,1879`
- Benchmark runs: 1 repeated token-id decode step after initial prompt
  next-token computation
- Profile mode: `--profile-benchmark-token`
- Execution policy: default only; Q8_K activation matvec disabled

Commands:

```sh
target/release/ferrite \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --prompt 'hello world' \
  --benchmark-runs 1 \
  --profile-benchmark-token

target/release/ferrite \
  --model target/models/qwen2.5-1.5b-instruct-q6_k.gguf \
  --prompt 'hello world' \
  --benchmark-runs 1 \
  --profile-benchmark-token
```

Both runs produced:

```text
next_token_id=198
benchmark_cached_tokens=3
benchmark_token_ids=9707
experimental_q8_k_activation_matvec=false
compare_q8_k_activation_matvec=false
q8_k_activation_matvec_policy=default_only
```

## Q8_0 Result

Summary:

```text
benchmark_total_ns=244639946
benchmark_avg_ns=244639946
profile_benchmark_token_total_ns=210712024
```

Role totals:

```text
profile_benchmark_token_role=ffn_down:Q8_0:1536:8960:14622720:53816351
profile_benchmark_token_role=ffn_gate:Q8_0:8960:1536:14622720:52166260
profile_benchmark_token_role=ffn_up:Q8_0:8960:1536:14622720:51901688
profile_benchmark_token_role=k_proj:Q8_0:256:1536:417792:1512130
profile_benchmark_token_role=o_proj:Q8_0:1536:1536:2506752:9002029
profile_benchmark_token_role=output:Q8_0:151936:1536:247959552:31787422
profile_benchmark_token_role=q_proj:Q8_0:1536:1536:2506752:9019010
profile_benchmark_token_role=v_proj:Q8_0:256:1536:417792:1507134
```

The Q8_0 profile is dominated by transformer-layer FFN roles. The final output
projection is significant, but much smaller than the combined FFN gate/up/down
cost.

## Q6_K Result

Summary:

```text
benchmark_total_ns=1323894241
benchmark_avg_ns=1323894241
profile_benchmark_token_total_ns=776450700
```

Role totals:

```text
profile_benchmark_token_role=ffn_down:Q6_K:1536:8960:11289600:131921617
profile_benchmark_token_role=ffn_gate:Q6_K:8960:1536:11289600:131895213
profile_benchmark_token_role=ffn_up:Q6_K:8960:1536:11289600:130168856
profile_benchmark_token_role=k_proj:Q6_K:256:1536:322560:4160388
profile_benchmark_token_role=o_proj:Q6_K:1536:1536:1935360:23195753
profile_benchmark_token_role=output:Q6_K:151936:1536:191439360:327882205
profile_benchmark_token_role=q_proj:Q6_K:1536:1536:1935360:23077710
profile_benchmark_token_role=v_proj:Q6_K:256:1536:322560:4148958
```

The Q6_K profile is dominated by the final output projection, followed by the
three transformer-layer FFN roles. Unlike local aarch64, this amd64 run shows
Q6_K's output projection as a primary bottleneck.

## Interpretation

The x86_64 AVX2 Q8_0 profile shows that Q8_0 remains mostly a transformer-layer
FFN optimization problem. A direct copy of prior naive row-level Rayon
scheduling is not justified because similar Q8_0 and Q5_0 shapes regressed in
earlier local experiments.

The x86_64 AVX2 Q6_K profile explains the large throughput gap: Q6_K is slower
than Q8_0 in FFN roles and dramatically slower in the final output projection.
The next Q6_K x86_64 implementation hypothesis should therefore start with the
Q6_K output projection path, not broad dispatch changes.

## Post Q6_K AVX2 Argmax Route

After `documentation/dev-notes/2026-06-28-q6-k-avx2-argmax.md`, the Q6_K
benchmark-token profile on the bounded x86_64 AVX2 pod reported:

```text
benchmark_total_ns=1115774978
benchmark_avg_ns=1115774978
profile_benchmark_token_total_ns=535634397
profile_benchmark_token_role=output:Q6_K:151936:1536:191439360:82575971
```

The output role improved from `327,882,205 ns` to `82,575,971 ns`, about a
74.8% reduction. Q6_K still remains below the Tier 1 throughput target because
the transformer-layer FFN roles remain large.

This profile does not justify promoting the experimental Q8_K activation matvec
path or changing default dispatch policy.

## Cleanup

The pod was deleted after the checks. A final cleanup check:

```sh
kubectl get pod ferrite-avx2-qwen15-q8q6-profile --context staging --ignore-not-found
```

returned no output.
