# Tier 1 AVX2 Qwen2.5 1.5B Q8_0 and Q6_K Model Check

Date: 2026-06-28

## Scope

This slice closes the x86_64 AVX2 model-output parity gap for the newly added
Qwen2.5-1.5B Q8_0 and Q6_K artifacts.

This is x86_64 AVX2 correctness evidence only. It does not prove throughput or
broader prompt coverage.

## Environment

Kubernetes context:

```text
staging
```

Bounded pod:

```text
name: ferrite-avx2-qwen15-q8q6
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

The pod built the copied source release CLI:

```text
Finished `release` profile [optimized] target(s) in 11.06s
```

Pod workspace size after source, models, and build:

```text
3.2G /work/ferrite
```

## Artifacts

The pod-side SHA256 hashes matched the local artifacts:

```text
e16d94f3b1eb243f6f6be9eee51090ef5dfd741324394fd5b6e0e425c33df5c7  target/models/qwen2.5-1.5b-instruct-q6_k.gguf
d7efb072e7724d25048a4fda0a3e10b04bdef5d06b1403a1c93bd9f1240a63c8  target/models/qwen2.5-1.5b-instruct-q8_0.gguf
```

## Checks

All checks used default execution:

```text
experimental_q8_k_activation_matvec=false
compare_q8_k_activation_matvec=false
q8_k_activation_matvec_policy=default_only
```

Q6_K matched the six local llama.cpp reference continuations:

```text
hello world -> 198,9707,11,1879,0,2585
The capital of France is -> 12095,13,576,6722,315,9625
Once upon a time -> 11,1052,572,264,3908,3743
Rust is a systems programming language -> 429,374,6188,311,387,6092
Machine learning models can -> 387,1483,311,7023,279,3853
The recipe calls for -> 220,17,25374,315,19828,323
```

Q8_0 matched the same six local llama.cpp reference continuations:

```text
hello world -> 198,9707,11,1879,0,2585
The capital of France is -> 12095,13,576,6722,315,9625
Once upon a time -> 11,1052,572,264,3908,3743
Rust is a systems programming language -> 429,374,6188,311,387,6092
Machine learning models can -> 387,1483,311,7023,279,3853
The recipe calls for -> 220,17,25374,315,19828,323
```

Every run reported:

```text
generated_match=true
match=true
```

Model byte counts:

```text
q6_k model_file_bytes=1464178720
q6_k scalar_weight_bytes=1458228224
q8_0 model_file_bytes=1894532128
q8_0 scalar_weight_bytes=1888581632
```

## Cleanup

The pod was deleted after the checks. A final cleanup check:

```sh
kubectl get pod ferrite-avx2-qwen15-q8q6 --context staging --ignore-not-found
```

returned no output.

## Conclusion

Qwen2.5-1.5B Q8_0 and Q6_K now have matching local aarch64 and x86_64 AVX2
six-prompt deterministic model-output evidence. Throughput and broader prompt
coverage remain separate Tier 1 gaps.
