# Tier 1 AVX2 Runtime Check

Date: 2026-06-28

## Scope

This note records a bounded x86_64 AVX2 runtime check for the Tier 1 SIMD
kernel surface.

This is focused backend evidence only. It does not prove real x86_64
model-output parity, x86_64 throughput, or full Tier 1 completion.

## Environment

Kubernetes safety checks:

```sh
kubectl config current-context
```

returned:

```text
staging
```

The local development host remained aarch64 macOS, so the runtime checks ran in
a bounded homelab pod. The node inventory showed two linux amd64 nodes:

```text
homelab-01   Ready   amd64   linux   k3s
homelab-02   Ready   amd64   linux   k3s
```

A disposable CPU feature pod reported:

```text
x86_64
```

and `/proc/cpuinfo` included both `avx` and `avx2`.

The Rust runtime pod used:

```text
image: rust:1.96-bookworm
cpu request: 500m
cpu limit: 2
memory request: 1Gi
memory limit: 4Gi
```

Source was copied into `/workspace/ferrite` with `.git` and `target` excluded.
The pod shell required `/usr/local/cargo/bin` to be added to `PATH`.

The runtime toolchain was:

```text
x86_64
rustc 1.96.0 (ac68faa20 2026-05-25)
host: x86_64-unknown-linux-gnu
cargo 1.96.0 (30a34c682 2026-05-25)
```

## Verification

Direct AVX2 backend-selection tests:

```sh
kubectl exec pod/ferrite-avx2-runtime -- sh -lc 'export PATH=/usr/local/cargo/bin:$PATH; cd /workspace/ferrite && uname -m && rustc -vV && cargo --version && cargo test -p ferrite-inference avx2 -- --nocapture'
```

passed:

```text
test scalar::matvec::tests::f32_matvec_uses_avx2_backend_on_x86_64 ... ok
test scalar::quantized_tests::q5_0_matvec_uses_avx2_backend_on_x86_64 ... ok
test scalar::quantized_tests::q4_k_matvec_uses_avx2_backend_on_x86_64 ... ok
test scalar::quantized_tests::q6_k_matvec_uses_avx2_backend_on_x86_64 ... ok
test scalar::quantized_tests::q8_0_matvec_uses_avx2_backend_on_x86_64 ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 28 filtered out
```

Q4_K and Q6_K row-order checks:

```sh
kubectl exec pod/ferrite-avx2-runtime -- sh -lc 'export PATH=/usr/local/cargo/bin:$PATH; cd /workspace/ferrite && cargo test -p ferrite-inference simd_matvec_preserves_parallel_row_order -- --nocapture'
```

passed:

```text
test scalar::quantized_tests::q4_k_simd_matvec_preserves_parallel_row_order ... ok
test scalar::quantized_tests::q6_k_simd_matvec_preserves_parallel_row_order ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 31 filtered out
```

Q8_0 direct argmax fast-path consistency:

```sh
kubectl exec pod/ferrite-avx2-runtime -- sh -lc 'export PATH=/usr/local/cargo/bin:$PATH; cd /workspace/ferrite && cargo test -p ferrite-inference q8_0_argmax_mul_vec_matches_full_matvec_argmax -- --nocapture'
```

passed:

```text
test scalar::quantized_tests::q8_0_argmax_mul_vec_matches_full_matvec_argmax ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 32 filtered out
```

The pod was deleted after the checks:

```sh
kubectl delete pod ferrite-avx2-runtime --wait=true
kubectl get pod ferrite-avx2-runtime --ignore-not-found
```

The final `get pod` command returned no output.

## Conclusion

Tier 1 now has focused x86_64 AVX2 runtime evidence for F32, Q8_0, Q5_0,
Q4_K, and Q6_K matvec backend selection, plus Q4_K/Q6_K row-order preservation
and Q8_0 argmax consistency.

This closes the previous compile-only AVX2 gap for focused kernel tests. It
does not close the real x86_64 model-output, x86_64 benchmark, or full-tier
throughput gaps.
