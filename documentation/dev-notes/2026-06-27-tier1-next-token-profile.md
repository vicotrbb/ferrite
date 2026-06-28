# 2026-06-27 Tier 1 Next-Token Profile

## Scope

This slice adds a focused scalar next-token profiling surface so Tier 1
optimization work can isolate hot operation labels before changing kernels.

It is diagnostic infrastructure, not a throughput optimization.

## Code Changes

- Added `ScalarProfileEvent` and `ProfiledNextToken` in a small scalar
  `profile` module.
- Added `ScalarLlamaSession::accept_token_profiled`.
- Added CLI flag `--profile-next-token`.
- The CLI profiles the final prompt-token forward pass that produces the
  printed `next_token_id`.
- Normal non-profiled inference keeps the existing direct `Matrix::mul_vec`
  path and avoids profiling timers and label formatting.

## TDD

Red test:

```sh
cargo test -p ferrite-cli cli_profiles_next_token_scalar_operations -- --nocapture
```

Expected failure:

```text
unknown argument --profile-next-token
```

Green test:

```sh
cargo test -p ferrite-cli cli_profiles_next_token_scalar_operations -- --nocapture
```

Result:

```text
test cli_profiles_next_token_scalar_operations ... ok
```

## Verification

Final workspace verification:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
git diff --check
```

All commands passed.

Additional targeted checks also passed while developing the slice:

```sh
cargo test -p ferrite-cli --test next_token_cli -- --nocapture
cargo test -p ferrite-inference --test scalar_reference -- --nocapture
cargo check -p ferrite-inference --target x86_64-unknown-linux-gnu --tests
```

## Real Model Probe

Release build:

```sh
cargo build --release -p ferrite-cli
```

Profile command:

```sh
target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt 'hello world' --profile-next-token
```

Representative output header:

```text
prompt_token_ids=28120,905
next_token_id=18
next_token="
profile_next_token_total_ns=181708088
```

The profile emitted per-operation labels such as:

```text
profile_next_token_op=layer.0.q_proj:545625
profile_next_token_op=layer.0.ffn_gate:1635084
profile_next_token_op=layer.0.ffn_down:1676041
profile_next_token_op=output:10108834
```

A later summarized profile pass reported:

```text
total_ns:213149624
ffn_up:54275375
ffn_down:44997749
ffn_gate:41418748
v_proj:22514085
q_proj:13867418
o_proj:13285207
k_proj:13284500
output:9506542
```

## Non-Profiled Benchmark Check

After adding profiling support, the normal non-profiled benchmark path was
checked again:

```sh
/usr/bin/time -l target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt 'hello world' --benchmark-runs 5
RAYON_NUM_THREADS=2 /usr/bin/time -l target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt 'hello world' --benchmark-runs 5
```

Results:

```text
benchmark_avg_ns=327864691
benchmark_avg_ns=546720408
```

These results remain in the retained Q4_K+Q6_K row-parallel range and do not
show the Q8_0/Q5_0 regression pattern that was reverted earlier.

## Result

Ferrite can now emit operation-level next-token profile labels for real Tier 1
models. The first SmolLM2-1.7B profile points at FFN matvec roles as the largest
aggregate operation group, with the output projection also visible as a single
hot label.

The next optimization slice should use this profile evidence instead of copying
row-level Rayon scheduling across formats blindly.
