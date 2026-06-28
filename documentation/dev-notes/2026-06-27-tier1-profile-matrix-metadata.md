# Tier 1 Profile Matrix Metadata

Date: 2026-06-27

## Scope

Add storage kind, matrix shape, and storage bytes to next-token profiling output
so optimization work can target exact tensor formats instead of operation labels
alone.

## Change

Commit `902e657` keeps the existing timing line:

```text
profile_next_token_op=<label>:<elapsed_ns>
```

It also emits one metadata line per profiled matvec:

```text
profile_next_token_matrix=<label>:<storage_kind>:<rows>:<cols>:<storage_bytes>
```

The normal non-profiled path still bypasses profile event allocation and calls
`Matrix::mul_vec` directly.

## TDD Evidence

Red:

```sh
cargo test -p ferrite-cli cli_profiles_next_token_scalar_operations -- --nocapture
```

Failed before implementation with:

```text
assertion failed: stdout.contains("profile_next_token_matrix=layer.0.q_proj:F32:2:2:16")
```

Green after implementation:

```text
test cli_profiles_next_token_scalar_operations ... ok
```

## Verification

Commands run before the implementation commit:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p ferrite-inference --target x86_64-unknown-linux-gnu --tests
git diff --check
```

All commands passed.

## Real Model Profile

Command:

```sh
target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt 'hello world' --profile-next-token
```

Representative emitted metadata:

```text
profile_next_token_matrix=layer.0.q_proj:Q4_K:2048:2048:2359296
profile_next_token_matrix=layer.0.ffn_gate:Q4_K:8192:2048:9437184
profile_next_token_matrix=layer.0.ffn_up:Q4_K:8192:2048:9437184
profile_next_token_matrix=layer.0.ffn_down:Q6_K:2048:8192:13762560
profile_next_token_matrix=output:Q6_K:49152:2048:82575360
```

Fresh parsed aggregate from one run:

```text
profile_next_token_total_ns=279307462
role_sum_ns=ffn_down:63372750
role_sum_ns=ffn_gate:62811957
role_sum_ns=ffn_up:61826002
role_sum_ns=o_proj:20340461
role_sum_ns=v_proj:20032832
role_sum_ns=k_proj:19333502
role_sum_ns=q_proj:19280083
role_sum_ns=output:12309875
```

## Non-Profile Benchmark Check

Default pool:

```text
benchmark_runs=5
benchmark_avg_ns=281364908
```

With `RAYON_NUM_THREADS=2`:

```text
benchmark_runs=5
benchmark_avg_ns=559712483
```

These are normal decode-path checks after adding profile metadata, not new
throughput pass evidence.

