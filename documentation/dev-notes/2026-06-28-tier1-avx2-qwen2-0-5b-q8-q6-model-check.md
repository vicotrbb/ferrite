# Tier 1 AVX2 Qwen2.5 0.5B Q8_0 and Q6_K Model Check

Date: 2026-06-28

## Scope

This note records bounded x86_64 AVX2 model-output checks for:

- `Qwen2.5-0.5B-Instruct-Q8_0.gguf`;
- `Qwen2.5-0.5B-Instruct-Q6_K.gguf`.

This extends the earlier local aarch64 Q8_0/Q6_K reference proofs to an
x86_64 AVX2 homelab pod. It does not prove Qwen2.5-1.5B additional
quantizations, SmolLM2 additional quantizations, broader prompt coverage, or
full-tier throughput.

## Environment

Kubernetes context:

```text
staging
```

The pod used:

```text
name: ferrite-avx2-qwen05-q8-q6
image: rust:1.96-bookworm
cpu request: 500m
cpu limit: 2
memory request: 1Gi
memory limit: 6Gi
node selector: kubernetes.io/arch=amd64
node: homelab-01
```

The runtime host reported:

```text
x86_64
```

and `/proc/cpuinfo` included `avx` and `avx2`.

Toolchain:

```text
rustc 1.96.0 (ac68faa20 2026-05-25)
host: x86_64-unknown-linux-gnu
cargo 1.96.0 (30a34c682 2026-05-25)
```

The copied model files were:

```text
Qwen2.5-0.5B-Instruct-Q8_0.gguf  507M
Qwen2.5-0.5B-Instruct-Q6_K.gguf  483M
```

Their pod-side SHA256 checks matched the local artifacts:

```text
25130a98aa782284a7dabea0c23245b2fd371ed47244e79d78b8ec23245fdf96  Qwen2.5-0.5B-Instruct-Q8_0.gguf
32c14c29a44712c02e29d5c2605593ece92ccb7a4358f56016a42b151434c842  Qwen2.5-0.5B-Instruct-Q6_K.gguf
```

The Kubernetes API returned transient `ServiceUnavailable: apiserver not ready`
errors during one copy attempt and one cleanup watch. Both recovered after a
short wait; no Ferrite command failed from model/runtime behavior.

## Build

The pod built the Linux x86_64 CLI from copied source:

```sh
kubectl exec pod/ferrite-avx2-qwen05-q8-q6 -- sh -lc \
  'export PATH=/usr/local/cargo/bin:$PATH; cd /workspace/ferrite && cargo build --release -p ferrite-cli'
```

Result:

```text
Finished `release` profile [optimized] target(s) in 45.16s
```

## Model-Output Checks

Both quantizations matched the same four deterministic six-token continuations
recorded in the local aarch64 notes.

Q8_0:

```sh
./target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q8_0.gguf --prompt "hello world" --generate-tokens 6 --expect-token-id 198 --expect-generated-token-ids 198,9707,11,4337,0,2585
./target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q8_0.gguf --prompt "The capital of France is" --generate-tokens 6 --expect-token-id 12095 --expect-generated-token-ids 12095,13,1084,374,279,7772
./target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q8_0.gguf --prompt "Once upon a time" --generate-tokens 6 --expect-token-id 11 --expect-generated-token-ids 11,1052,572,264,3908,883
./target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q8_0.gguf --prompt "Rust is a systems programming language" --generate-tokens 6 --expect-token-id 429 --expect-generated-token-ids 429,374,6188,311,387,6092
```

Q6_K:

```sh
./target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q6_K.gguf --prompt "hello world" --generate-tokens 6 --expect-token-id 198 --expect-generated-token-ids 198,9707,11,4337,0,2585
./target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q6_K.gguf --prompt "The capital of France is" --generate-tokens 6 --expect-token-id 12095 --expect-generated-token-ids 12095,13,1084,374,279,7772
./target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q6_K.gguf --prompt "Once upon a time" --generate-tokens 6 --expect-token-id 11 --expect-generated-token-ids 11,1052,572,264,3908,883
./target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q6_K.gguf --prompt "Rust is a systems programming language" --generate-tokens 6 --expect-token-id 429 --expect-generated-token-ids 429,374,6188,311,387,6092
```

All eight runs reported:

```text
generated_match=true
match=true
q8_k_activation_matvec_policy=default_only
```

## Benchmark Checks

Bounded `hello world` benchmark runs were captured for context.

Q8_0 default pool:

```text
benchmark_runs=5
benchmark_avg_ns=71197566
benchmark_token_ids=9707,11,4337,0,2585
```

Q8_0 with `RAYON_NUM_THREADS=2`:

```text
benchmark_runs=5
benchmark_avg_ns=73319573
benchmark_token_ids=9707,11,4337,0,2585
```

Q6_K default pool:

```text
benchmark_runs=5
benchmark_avg_ns=96172191
benchmark_token_ids=9707,11,4337,0,2585
```

Q6_K with `RAYON_NUM_THREADS=2`:

```text
benchmark_runs=5
benchmark_avg_ns=102300612
benchmark_token_ids=9707,11,4337,0,2585
```

These are bounded single-prompt measurements only. They do not prove full Tier
1 throughput.

## Cleanup

The pod was deleted after the run. A final cleanup check:

```sh
kubectl get pod ferrite-avx2-qwen05-q8-q6 --ignore-not-found
```

returned no output.

## Conclusion

Qwen2.5-0.5B Q8_0 and Q6_K now have x86_64 AVX2 model-output parity evidence
for the four fixed Tier 1 Qwen2 prompts. Broader prompt coverage, additional
1.5B quantizations, and full x86_64 throughput remain open.
