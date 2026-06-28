# Tier 1 AVX2 Qwen2.5 0.5B Model Check

Date: 2026-06-28

## Scope

This note records a bounded x86_64 AVX2 real-model check for
Qwen2.5-0.5B-Instruct Q4_K_M.

This proves one fixed Tier 1 Qwen2 model-output profile on an x86_64 AVX2
homelab pod. It does not prove broader x86_64 model coverage, SmolLM2 x86_64
parity, Qwen2.5-1.5B x86_64 parity, or full-tier throughput.

## Environment

Kubernetes context:

```text
staging
```

The pod used:

```text
image: rust:1.96-bookworm
cpu request: 500m
cpu limit: 2
memory request: 1Gi
memory limit: 6Gi
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

## Model-Output Check

Command:

```sh
kubectl exec pod/ferrite-avx2-model -- sh -lc 'export PATH=/usr/local/cargo/bin:$PATH; cd /workspace/ferrite && uname -m && grep -m1 flags /proc/cpuinfo && ls -lh target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf && cargo run --release -p ferrite-cli -- --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --prompt "hello world" --generate-tokens 3 --expect-token-id 198 --expect-generated-token-ids 198,9707,11'
```

Result:

```text
prompt_token_ids=14990,1879
experimental_q8_k_activation_matvec=false
compare_q8_k_activation_matvec=false
next_token_id=198
generated_token_ids=198,9707,11
generated_text=
Hello,
expected_generated_token_ids=198,9707,11
generated_match=true
model_file_bytes=397808192
model_file_retained_bytes=0
scalar_weight_bytes=391859712
kv_cache_bytes=122880
expected_token_id=198
match=true
```

## Benchmark Checks

Default Rayon pool:

```sh
kubectl exec pod/ferrite-avx2-model -- sh -lc 'export PATH=/usr/local/cargo/bin:$PATH; cd /workspace/ferrite && target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --prompt "hello world" --benchmark-runs 5'
```

returned:

```text
benchmark_runs=5
benchmark_cached_tokens=7
benchmark_token_ids=9707,11,1879,0,2585
benchmark_total_ns=1578730848
benchmark_avg_ns=315746169
```

This is about 3.17 tok/s, below the 10 tok/s Tier 1 throughput target.

Two Rayon threads:

```sh
kubectl exec pod/ferrite-avx2-model -- sh -lc 'export PATH=/usr/local/cargo/bin:$PATH; cd /workspace/ferrite && RAYON_NUM_THREADS=2 target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --prompt "hello world" --benchmark-runs 5'
```

returned:

```text
benchmark_runs=5
benchmark_cached_tokens=7
benchmark_token_ids=9707,11,1879,0,2585
benchmark_total_ns=1581427531
benchmark_avg_ns=316285506
```

This is about 3.16 tok/s, also below target.

The pod was deleted after the run, and `kubectl get pod ferrite-avx2-model
--ignore-not-found` returned no output.

## Conclusion

Ferrite matched the documented Qwen2.5-0.5B Q4_K_M `hello world` next-token and
three-token deterministic continuation on x86_64 AVX2.

The same bounded pod did not meet the Tier 1 throughput target for this model.
That keeps x86_64 throughput open even though this fixed model-output check
passed.
