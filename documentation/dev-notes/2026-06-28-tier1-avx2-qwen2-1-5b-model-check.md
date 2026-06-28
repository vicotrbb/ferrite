# Tier 1 AVX2 Qwen2.5 1.5B Model Check

Date: 2026-06-28

## Scope

This note records x86_64 AVX2 model-output and benchmark evidence for
Qwen2.5-1.5B-Instruct Q4_K_M.

It proves the same three fixed Qwen2 prompts already used by the local aarch64
Tier 1 proof set:

- `hello world`
- `The capital of France is`
- `Once upon a time`

This does not prove SmolLM2 x86_64 parity, additional quantizations, broader
prompt coverage, or full-tier throughput.

## Environment

Kubernetes context:

```text
staging
```

The bounded pod used:

```text
name: ferrite-avx2-qwen15-http
image: rust:1.96-bookworm
cpu request: 500m
cpu limit: 2
memory request: 1Gi
memory limit: 7Gi
```

The runtime host reported:

```text
x86_64
```

and `/proc/cpuinfo` included `avx` and `avx2`.

The runtime toolchain was:

```text
rustc 1.96.0 (ac68faa20 2026-05-25)
host: x86_64-unknown-linux-gnu
```

## Model Transfer

The previous single-stream 1GB `kubectl exec` upload failed during an earlier
attempt. This run transferred the model in 64 MiB chunks and reassembled it in
the pod.

Local checksum:

```text
6a1a2eb6d15622bf3c96857206351ba97e1af16c30d7a74ee38970e434e9407e  target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf
```

Pod checksum after reassembly:

```text
6a1a2eb6d15622bf3c96857206351ba97e1af16c30d7a74ee38970e434e9407e  qwen2.5-1.5b-instruct-q4_k_m.gguf
```

The reassembled model size was:

```text
1.1G
```

## Model-Output Checks

`hello world`:

```sh
kubectl exec pod/ferrite-avx2-qwen15-http -- sh -lc 'export PATH=/usr/local/cargo/bin:$PATH; cd /workspace/ferrite && uname -m && grep -m1 flags /proc/cpuinfo && rustc -vV && cargo run --release -p ferrite-cli -- --model target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf --prompt "hello world" --generate-tokens 3 --expect-generated-token-ids 198,9707,11'
```

passed:

```text
prompt_token_ids=14990,1879
experimental_q8_k_activation_matvec=false
compare_q8_k_activation_matvec=false
q8_k_activation_matvec_policy=default_only
next_token_id=198
generated_token_ids=198,9707,11
generated_text=
Hello,
expected_generated_token_ids=198,9707,11
generated_match=true
model_file_bytes=1117320736
model_file_retained_bytes=0
scalar_weight_bytes=1111370240
kv_cache_bytes=286720
```

`The capital of France is`:

```sh
kubectl exec pod/ferrite-avx2-qwen15-http -- sh -lc 'cd /workspace/ferrite && target/release/ferrite --model target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf --prompt "The capital of France is" --generate-tokens 3 --expect-generated-token-ids 12095,13,576'
```

passed:

```text
prompt_token_ids=785,6722,315,9625,374
experimental_q8_k_activation_matvec=false
compare_q8_k_activation_matvec=false
q8_k_activation_matvec_policy=default_only
next_token_id=12095
generated_token_ids=12095,13,576
generated_text= Paris. The
expected_generated_token_ids=12095,13,576
generated_match=true
model_file_bytes=1117320736
model_file_retained_bytes=0
scalar_weight_bytes=1111370240
kv_cache_bytes=458752
```

`Once upon a time`:

```sh
kubectl exec pod/ferrite-avx2-qwen15-http -- sh -lc 'cd /workspace/ferrite && target/release/ferrite --model target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf --prompt "Once upon a time" --generate-tokens 3 --expect-generated-token-ids 11,1052,572'
```

passed:

```text
prompt_token_ids=12522,5193,264,882
experimental_q8_k_activation_matvec=false
compare_q8_k_activation_matvec=false
q8_k_activation_matvec_policy=default_only
next_token_id=11
generated_token_ids=11,1052,572
generated_text=, there was
expected_generated_token_ids=11,1052,572
generated_match=true
model_file_bytes=1117320736
model_file_retained_bytes=0
scalar_weight_bytes=1111370240
kv_cache_bytes=401408
```

## Benchmark Checks

`hello world`, default Rayon pool:

```sh
kubectl exec pod/ferrite-avx2-qwen15-http -- sh -lc 'cd /workspace/ferrite && target/release/ferrite --model target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf --prompt "hello world" --benchmark-runs 5'
```

returned:

```text
benchmark_runs=5
benchmark_cached_tokens=7
benchmark_token_ids=9707,11,1879,0,1096
benchmark_total_ns=8981071848
benchmark_avg_ns=1796214369
```

This is about 0.56 tok/s, below the 10 tok/s Tier 1 throughput target.

`hello world`, two Rayon threads:

```sh
kubectl exec pod/ferrite-avx2-qwen15-http -- sh -lc 'cd /workspace/ferrite && RAYON_NUM_THREADS=2 target/release/ferrite --model target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf --prompt "hello world" --benchmark-runs 5'
```

returned:

```text
benchmark_runs=5
benchmark_cached_tokens=7
benchmark_token_ids=9707,11,1879,0,1096
benchmark_total_ns=8924427764
benchmark_avg_ns=1784885552
```

This is about 0.56 tok/s, also below target.

The pod was deleted after the checks, and:

```sh
kubectl get pod ferrite-avx2-qwen15-http --ignore-not-found
```

returned no output.

## Conclusion

Qwen2.5-1.5B Q4_K_M now has x86_64 AVX2 model-output evidence for all three
fixed Tier 1 Qwen2 prompts.

Throughput on the bounded x86_64 pod remains far below the Tier 1 target. The
local aarch64 Q8_K opt-in benchmark remains the better Qwen2.5-1.5B performance
signal for now, but that path is still not default eligible because SmolLM2
parity fails.
