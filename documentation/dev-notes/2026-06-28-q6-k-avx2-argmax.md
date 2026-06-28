# Q6_K AVX2 Argmax Route

Date: 2026-06-28

## Scope

This slice adds an x86_64 AVX2 route for Q6_K token-id argmax matvecs.

The prior x86_64 Qwen2.5-1.5B Q6_K benchmark-token profile showed the final
Q6_K output projection as the largest single role:

```text
profile_benchmark_token_role=output:Q6_K:151936:1536:191439360:327882205
```

The output projection uses `Matrix::argmax_mul_vec_with_options` in the
token-id path. Before this slice, Q6_K argmax used NEON on aarch64 but fell
back to scalar row scoring on x86_64.

## Implementation

- Added `avx2_q6_k_argmax_mul_vec` in
  `crates/ferrite-inference/src/scalar/q6_k_avx2.rs`.
- Routed x86_64 Q6_K argmax calls through that helper when AVX2 is detected.
- Kept the existing decoded-block plus AVX2 f32 dot shape for this first slice.
  It does not yet implement a fused Q6_K integer dot.

## TDD Evidence

Red test:

```sh
cargo test -p ferrite-inference --target x86_64-unknown-linux-gnu \
  avx2_q6_k_argmax_mul_vec_matches_full_matvec_argmax --no-run
```

Expected failure:

```text
error[E0432]: unresolved import `super::avx2_q6_k_argmax_mul_vec`
```

Green checks:

```sh
cargo check -p ferrite-inference --target x86_64-unknown-linux-gnu --tests
cargo test -p ferrite-inference q6_k_argmax_mul_vec_matches_full_matvec_argmax -- --nocapture
cargo test -p ferrite-inference q6_k -- --nocapture
```

All three passed locally. The cross-target `cargo test --no-run` still cannot
link Linux x86_64 test binaries from macOS because the host linker rejects GNU
linker flags, so runtime x86_64 verification used a bounded staging pod.

## x86_64 Runtime Evidence

Kubernetes context:

```text
staging
```

Bounded pod:

```text
name: ferrite-avx2-q6-argmax
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

The pod reported `x86_64`, `/proc/cpuinfo` included `avx2`, and the pod-side
toolchain was:

```text
cargo 1.96.0 (30a34c682 2026-05-25)
rustc 1.96.0 (ac68faa20 2026-05-25)
host: x86_64-unknown-linux-gnu
LLVM version: 22.1.2
```

The new x86_64 test passed in the pod:

```sh
cargo test -p ferrite-inference \
  avx2_q6_k_argmax_mul_vec_matches_full_matvec_argmax -- --nocapture
```

Result:

```text
test scalar::q6_k_avx2::tests::avx2_q6_k_argmax_mul_vec_matches_full_matvec_argmax ... ok
```

The pod-side Q6_K model hash matched the local artifact:

```text
e16d94f3b1eb243f6f6be9eee51090ef5dfd741324394fd5b6e0e425c33df5c7  target/models/qwen2.5-1.5b-instruct-q6_k.gguf
```

The release CLI built successfully in the pod:

```text
Finished `release` profile [optimized] target(s) in 5.16s
```

## Benchmark Evidence

After this slice, the Qwen2.5-1.5B Q6_K benchmark-token profile reported:

```text
benchmark_total_ns=1115774978
benchmark_avg_ns=1115774978
profile_benchmark_token_total_ns=535634397
profile_benchmark_token_role=output:Q6_K:151936:1536:191439360:82575971
```

Compared with the prior x86_64 profile's output role of `327,882,205 ns`, the
Q6_K output projection role improved by about 74.8%.

The bounded five-token benchmark reported:

```text
benchmark_avg_ns=1086706040
```

or about 0.92 tok/s on the default pool. The prior comparable default-pool run
was `1,356,563,424 ns`, about 0.74 tok/s.

With `RAYON_NUM_THREADS=2`, the benchmark reported:

```text
benchmark_avg_ns=1111777214
```

or about 0.90 tok/s. The prior comparable two-thread run was
`1,348,577,646 ns`, about 0.74 tok/s.

The pod cgroup reported:

```text
memory.peak=4677971968
memory.max=8589934592
```

## Conclusion

The x86_64 AVX2 Q6_K argmax route materially improves the profiled output
projection and the real Qwen2.5-1.5B Q6_K benchmark-token loop. It does not
make Q6_K a Tier 1 throughput pass: the model remains below 10 tok/s, and the
remaining profile is dominated by transformer-layer Q6_K FFN roles.

This slice does not change Q8_K activation-matvec policy and does not justify
broad dispatch changes.

## Cleanup

The pod was deleted after the checks. A final cleanup check:

```sh
kubectl get pod ferrite-avx2-q6-argmax --context staging --ignore-not-found
```

returned no output.
