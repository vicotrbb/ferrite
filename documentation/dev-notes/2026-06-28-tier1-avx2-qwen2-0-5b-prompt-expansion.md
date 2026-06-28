# Tier 1 AVX2 Qwen2.5 0.5B Prompt Expansion

Date: 2026-06-28

## Scope

This note expands x86_64 AVX2 model-output evidence for
Qwen2.5-0.5B-Instruct Q4_K_M beyond the earlier `hello world` profile.

It proves the two additional fixed prompts already used by local aarch64 Tier 1
coverage:

- `The capital of France is`
- `Once upon a time`

This is still scoped to Qwen2.5-0.5B Q4_K_M. It does not prove Qwen2.5-1.5B
x86_64 parity, SmolLM2 x86_64 parity, additional quantizations, or full-tier
throughput.

## Environment

Kubernetes context:

```text
staging
```

The bounded pod used:

```text
name: ferrite-avx2-qwen05-prompts
image: rust:1.96-bookworm
cpu request: 500m
cpu limit: 2
memory request: 1Gi
memory limit: 4Gi
```

The runtime host reported:

```text
x86_64
```

and `/proc/cpuinfo` included `avx` and `avx2`.

The copied model was:

```text
target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf
model file size: 380M
```

The runtime toolchain was:

```text
rustc 1.96.0 (ac68faa20 2026-05-25)
host: x86_64-unknown-linux-gnu
```

## Model-Output Checks

France prompt:

```sh
kubectl exec pod/ferrite-avx2-qwen05-prompts -- sh -lc 'export PATH=/usr/local/cargo/bin:$PATH; cd /workspace/ferrite && uname -m && grep -m1 flags /proc/cpuinfo && rustc -vV && ls -lh target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf && cargo run --release -p ferrite-cli -- --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --prompt "The capital of France is" --generate-tokens 3 --expect-generated-token-ids 12095,13,1084'
```

passed:

```text
prompt_token_ids=785,6722,315,9625,374
experimental_q8_k_activation_matvec=false
compare_q8_k_activation_matvec=false
q8_k_activation_matvec_policy=default_only
next_token_id=12095
generated_token_ids=12095,13,1084
generated_text= Paris. It
expected_generated_token_ids=12095,13,1084
generated_match=true
model_file_bytes=397808192
model_file_retained_bytes=0
scalar_weight_bytes=391859712
kv_cache_bytes=196608
```

Third prompt:

```sh
kubectl exec pod/ferrite-avx2-qwen05-prompts -- sh -lc 'cd /workspace/ferrite && target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --prompt "Once upon a time" --generate-tokens 3 --expect-generated-token-ids 11,1052,572'
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
model_file_bytes=397808192
model_file_retained_bytes=0
scalar_weight_bytes=391859712
kv_cache_bytes=172032
```

## Benchmark Checks

France prompt, default Rayon pool:

```sh
kubectl exec pod/ferrite-avx2-qwen05-prompts -- sh -lc 'cd /workspace/ferrite && target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --prompt "The capital of France is" --benchmark-runs 5'
```

returned:

```text
benchmark_runs=5
benchmark_cached_tokens=10
benchmark_token_ids=13,1084,374,279,7772
benchmark_total_ns=1843861627
benchmark_avg_ns=368772325
```

This is about 2.71 tok/s, below the 10 tok/s Tier 1 throughput target.

France prompt, two Rayon threads:

```sh
kubectl exec pod/ferrite-avx2-qwen05-prompts -- sh -lc 'cd /workspace/ferrite && RAYON_NUM_THREADS=2 target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --prompt "The capital of France is" --benchmark-runs 5'
```

returned:

```text
benchmark_runs=5
benchmark_cached_tokens=10
benchmark_token_ids=13,1084,374,279,7772
benchmark_total_ns=1798442778
benchmark_avg_ns=359688555
```

This is about 2.78 tok/s, also below target.

The pod was deleted after the checks, and:

```sh
kubectl get pod ferrite-avx2-qwen05-prompts ferrite-avx2-qwen15 --ignore-not-found
```

returned no output.

## Conclusion

Qwen2.5-0.5B Q4_K_M now has x86_64 AVX2 model-output evidence for all three
fixed Tier 1 prompts used by the local aarch64 proof set.

Throughput on the bounded x86_64 pod remains below the Tier 1 target.
