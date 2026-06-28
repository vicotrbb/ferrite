# Tier 1 AVX2 SmolLM2 1.7B Model Check

Date: 2026-06-28

## Scope

This note records x86_64 AVX2 model-output and benchmark evidence for
SmolLM2-1.7B-Instruct Q4_K_M.

It proves the same three fixed SmolLM2 prompts already used by the local aarch64
Tier 1 proof set:

- `hello world`
- `The capital of France is`
- `Once upon a time`

This checks default execution only. The experimental Q4_K/Q6_K x Q8_K
activation matvec path is still not default eligible for SmolLM2 because the
documented opt-in path diverges on this model.

## Environment

Kubernetes context:

```text
staging
```

The bounded pod used:

```text
name: ferrite-avx2-smollm17
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

The model was transferred in 64 MiB chunks and reassembled in the pod.

Local checksum:

```text
77665ea4815999596525c636fbeb56ba8b080b46ae85efef4f0d986a139834d7  target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf
```

Pod checksum after reassembly:

```text
77665ea4815999596525c636fbeb56ba8b080b46ae85efef4f0d986a139834d7  SmolLM2-1.7B-Instruct-Q4_K_M.gguf
```

The reassembled model size was:

```text
1007M
```

## Model-Output Checks

`hello world`:

```sh
kubectl exec pod/ferrite-avx2-smollm17 -- sh -lc 'export PATH=/usr/local/cargo/bin:$PATH; cd /workspace/ferrite && uname -m && grep -m1 flags /proc/cpuinfo && rustc -vV && cargo run --release -p ferrite-cli -- --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt "hello world" --generate-tokens 6 --expect-token-id 18 --expect-generated-token-ids 18,198,3725,198,198,788'
```

passed:

```text
prompt_token_ids=28120,905
experimental_q8_k_activation_matvec=false
compare_q8_k_activation_matvec=false
q8_k_activation_matvec_policy=default_only
next_token_id=18
generated_token_ids=18,198,3725,198,198,788
expected_generated_token_ids=18,198,3725,198,198,788
generated_match=true
model_file_bytes=1055609824
model_file_retained_bytes=0
scalar_weight_bytes=1053827072
kv_cache_bytes=3145728
expected_token_id=18
match=true
```

`The capital of France is`:

```sh
kubectl exec pod/ferrite-avx2-smollm17 -- sh -lc 'cd /workspace/ferrite && target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt "The capital of France is" --generate-tokens 6 --expect-generated-token-ids 7042,30,2'
```

passed:

```text
prompt_token_ids=504,3575,282,4649,314
experimental_q8_k_activation_matvec=false
compare_q8_k_activation_matvec=false
q8_k_activation_matvec_policy=default_only
next_token_id=7042
generated_token_ids=7042,30,2
generated_stopped_on_eos=true
generated_text= Paris.<|im_end|>
expected_generated_token_ids=7042,30,2
generated_match=true
model_file_bytes=1055609824
model_file_retained_bytes=0
scalar_weight_bytes=1053827072
kv_cache_bytes=2752512
```

`Once upon a time`:

```sh
kubectl exec pod/ferrite-avx2-smollm17 -- sh -lc 'cd /workspace/ferrite && target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt "Once upon a time" --generate-tokens 6 --expect-generated-token-ids 28,281,253,1165,6560,32047'
```

passed:

```text
prompt_token_ids=6403,1980,253,655
experimental_q8_k_activation_matvec=false
compare_q8_k_activation_matvec=false
q8_k_activation_matvec_policy=default_only
next_token_id=28
generated_token_ids=28,281,253,1165,6560,32047
generated_text=, in a small village nestled
expected_generated_token_ids=28,281,253,1165,6560,32047
generated_match=true
model_file_bytes=1055609824
model_file_retained_bytes=0
scalar_weight_bytes=1053827072
kv_cache_bytes=3932160
```

## Benchmark Checks

`hello world`, default Rayon pool:

```sh
kubectl exec pod/ferrite-avx2-smollm17 -- sh -lc 'cd /workspace/ferrite && target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt "hello world" --benchmark-runs 5'
```

returned:

```text
benchmark_runs=5
benchmark_cached_tokens=7
benchmark_token_ids=198,3725,198,198,788
benchmark_total_ns=7703126700
benchmark_avg_ns=1540625340
```

This is about 0.65 tok/s, below the 10 tok/s Tier 1 throughput target.

`hello world`, two Rayon threads:

```sh
kubectl exec pod/ferrite-avx2-smollm17 -- sh -lc 'cd /workspace/ferrite && RAYON_NUM_THREADS=2 target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt "hello world" --benchmark-runs 5'
```

returned:

```text
benchmark_runs=5
benchmark_cached_tokens=7
benchmark_token_ids=198,3725,198,198,788
benchmark_total_ns=7736220883
benchmark_avg_ns=1547244176
```

This is about 0.65 tok/s, also below target.

The pod was deleted after the checks, and:

```sh
kubectl get pod ferrite-avx2-smollm17 --ignore-not-found
```

returned no output.

## Conclusion

SmolLM2-1.7B Q4_K_M now has x86_64 AVX2 model-output evidence for all three
fixed Tier 1 SmolLM2 prompts.

Throughput on the bounded x86_64 pod remains far below the Tier 1 target. This
does not change the Q8_K policy: the default path matches, while the
experimental Q8_K activation path remains opt-in because it diverges on SmolLM2.
