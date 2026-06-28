# Tier 1 AVX2 Fixed Prompt Closure

Date: 2026-06-28

## Scope

This slice closes the remaining x86_64 AVX2 fixed-prompt gaps for the current
local Tier 1 prompt set:

- Q4_K_M fourth prompt: `Rust is a systems programming language`;
- sixth prompt: `The recipe calls for`.

It covers the current local Tier 1 model/quantization set where applicable:

- `SmolLM2-1.7B-Instruct-Q4_K_M.gguf`;
- `Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`;
- `qwen2.5-1.5b-instruct-q4_k_m.gguf`;
- `Qwen2.5-0.5B-Instruct-Q8_0.gguf`;
- `Qwen2.5-0.5B-Instruct-Q6_K.gguf`.

This is x86_64 AVX2 model-output parity evidence only. It does not add broader
prompt coverage, additional 1.5B quantizations, or throughput proof.

## Environment

Kubernetes context:

```text
staging
```

The bounded pod used:

```text
name: ferrite-avx2-prompt-closure
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
Finished `release` profile [optimized] target(s) in 10.50s
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

## Q4_K_M Fourth-Prompt Checks

SmolLM2-1.7B Q4_K_M:

```sh
target/release/ferrite \
  --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf \
  --prompt "Rust is a systems programming language" \
  --generate-tokens 6 \
  --expect-token-id 338 \
  --expect-generated-token-ids 338,2433,253,1837,3500,1743
```

Qwen2.5-0.5B Q4_K_M:

```sh
target/release/ferrite \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --prompt "Rust is a systems programming language" \
  --generate-tokens 3 \
  --expect-token-id 429 \
  --expect-generated-token-ids 429,374,6188
```

Qwen2.5-1.5B Q4_K_M:

```sh
target/release/ferrite \
  --model target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf \
  --prompt "Rust is a systems programming language" \
  --generate-tokens 3 \
  --expect-token-id 429 \
  --expect-generated-token-ids 429,374,6188
```

All three runs reported:

```text
q8_k_activation_matvec_policy=default_only
generated_match=true
match=true
```

## Sixth-Prompt Checks

SmolLM2-1.7B Q4_K_M:

```sh
target/release/ferrite \
  --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf \
  --prompt "The recipe calls for" \
  --generate-tokens 6 \
  --expect-token-id 216 \
  --expect-generated-token-ids 216,34,12382,282,7367,30
```

Qwen2.5-0.5B Q4_K_M:

```sh
target/release/ferrite \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --prompt "The recipe calls for" \
  --generate-tokens 6 \
  --expect-token-id 220 \
  --expect-generated-token-ids 220,18,25374,315,19828,13
```

Qwen2.5-1.5B Q4_K_M:

```sh
target/release/ferrite \
  --model target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf \
  --prompt "The recipe calls for" \
  --generate-tokens 6 \
  --expect-token-id 220 \
  --expect-generated-token-ids 220,17,25374,315,19828,323
```

Qwen2.5-0.5B Q8_0:

```sh
target/release/ferrite \
  --model target/models/Qwen2.5-0.5B-Instruct-Q8_0.gguf \
  --prompt "The recipe calls for" \
  --generate-tokens 6 \
  --expect-token-id 220 \
  --expect-generated-token-ids 220,18,25374,315,19828,13
```

Qwen2.5-0.5B Q6_K:

```sh
target/release/ferrite \
  --model target/models/Qwen2.5-0.5B-Instruct-Q6_K.gguf \
  --prompt "The recipe calls for" \
  --generate-tokens 6 \
  --expect-token-id 220 \
  --expect-generated-token-ids 220,18,25374,315,19828,13
```

All five runs reported:

```text
q8_k_activation_matvec_policy=default_only
generated_match=true
match=true
```

## Cleanup

The pod was deleted after the checks. A final cleanup check:

```sh
kubectl get pod ferrite-avx2-prompt-closure --ignore-not-found
```

returned no output.

## Conclusion

The current fixed six-prompt Tier 1 local artifact set now has matching
x86_64 AVX2 model-output evidence for the same prompt coverage. Broader prompt
coverage, additional 1.5B quantizations, and full Tier 1 throughput remain open.
