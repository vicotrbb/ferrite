# Tier 1 AVX2 Fifth Prompt Reference Check

Date: 2026-06-28

## Scope

This slice extends x86_64 AVX2 model-output evidence for the fifth fixed local
Tier 1 prompt:

```text
Machine learning models can
```

It covers the current local Tier 1 model/quantization set:

- `SmolLM2-1.7B-Instruct-Q4_K_M.gguf`;
- `Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`;
- `qwen2.5-1.5b-instruct-q4_k_m.gguf`;
- `Qwen2.5-0.5B-Instruct-Q8_0.gguf`;
- `Qwen2.5-0.5B-Instruct-Q6_K.gguf`.

This is x86_64 AVX2 prompt-parity evidence only. It does not add sixth-prompt
x86_64 coverage, Q4_K_M fourth-prompt x86_64 coverage, broader prompts,
additional 1.5B quantizations, or full-tier throughput.

## Environment

Kubernetes context:

```text
staging
```

The bounded pod used:

```text
name: ferrite-avx2-fifth-prompt
image: rust:1.96-bookworm
cpu request: 500m
cpu limit: 2
memory request: 1Gi
memory limit: 8Gi
ephemeral-storage request: 8Gi
ephemeral-storage limit: 14Gi
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
```

The pod built the release CLI from copied source:

```text
Finished `release` profile [optimized] target(s) in 8.65s
```

## Model Artifacts

The pod-side SHA256 hashes matched the local artifacts:

```text
77665ea4815999596525c636fbeb56ba8b080b46ae85efef4f0d986a139834d7  SmolLM2-1.7B-Instruct-Q4_K_M.gguf
6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653  Qwen2.5-0.5B-Instruct-Q4_K_M.gguf
6a1a2eb6d15622bf3c96857206351ba97e1af16c30d7a74ee38970e434e9407e  qwen2.5-1.5b-instruct-q4_k_m.gguf
25130a98aa782284a7dabea0c23245b2fd371ed47244e79d78b8ec23245fdf96  Qwen2.5-0.5B-Instruct-Q8_0.gguf
32c14c29a44712c02e29d5c2605593ece92ccb7a4358f56016a42b151434c842  Qwen2.5-0.5B-Instruct-Q6_K.gguf
```

## Model-Output Checks

SmolLM2-1.7B Q4_K_M:

```sh
target/release/ferrite \
  --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf \
  --prompt "Machine learning models can" \
  --generate-tokens 6 \
  --expect-token-id 597 \
  --expect-generated-token-ids 597,325,804,288,6524,260
```

Qwen2.5-0.5B Q4_K_M:

```sh
target/release/ferrite \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --prompt "Machine learning models can" \
  --generate-tokens 6 \
  --expect-token-id 387 \
  --expect-generated-token-ids 387,1483,311,7023,279,28636
```

Qwen2.5-1.5B Q4_K_M:

```sh
target/release/ferrite \
  --model target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf \
  --prompt "Machine learning models can" \
  --generate-tokens 6 \
  --expect-token-id 387 \
  --expect-generated-token-ids 387,1483,311,7023,279,28636
```

Qwen2.5-0.5B Q8_0:

```sh
target/release/ferrite \
  --model target/models/Qwen2.5-0.5B-Instruct-Q8_0.gguf \
  --prompt "Machine learning models can" \
  --generate-tokens 6 \
  --expect-token-id 387 \
  --expect-generated-token-ids 387,1483,311,7023,279,28636
```

Qwen2.5-0.5B Q6_K:

```sh
target/release/ferrite \
  --model target/models/Qwen2.5-0.5B-Instruct-Q6_K.gguf \
  --prompt "Machine learning models can" \
  --generate-tokens 6 \
  --expect-token-id 387 \
  --expect-generated-token-ids 387,1483,311,7023,279,28636
```

Every run reported:

```text
q8_k_activation_matvec_policy=default_only
generated_match=true
match=true
```

## Cleanup

The pod was deleted after the checks. A final cleanup check:

```sh
kubectl get pod ferrite-avx2-fifth-prompt --ignore-not-found
```

returned no output.

## Conclusion

The fifth fixed prompt now has x86_64 AVX2 model-output parity evidence for the
current local Tier 1 model/quantization set. Qwen2.5-0.5B Q8_0 and Q6_K now
have x86_64 AVX2 evidence for the first five fixed Qwen2 prompts. Q4_K_M
artifacts now have x86_64 AVX2 evidence for the first three fixed prompts plus
the fifth prompt; their fourth prompt remains unproven on x86_64 AVX2.
