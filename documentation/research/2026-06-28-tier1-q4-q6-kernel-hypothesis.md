# Tier 1 Q4_K/Q6_K Kernel Hypothesis

Date: 2026-06-28

## Context

The latest Qwen2.5-1.5B Q4_K_M benchmark-token profile still misses the Tier 1
throughput target:

```text
benchmark_avg_ns=296499736
```

The hot aggregate roles remain concentrated in Q4_K FFN gate/up, Q4_K/Q6_K FFN
down, and the Q6_K output projection:

```text
profile_benchmark_token_role=ffn_gate:Q4_K:8960:1536:7741440:26667421
profile_benchmark_token_role=ffn_up:Q4_K:8960:1536:7741440:27393334
profile_benchmark_token_role=ffn_down:Q4_K:1536:8960:7741440:12245749
profile_benchmark_token_role=ffn_down:Q6_K:1536:8960:11289600:17353041
profile_benchmark_token_role=output:Q6_K:151936:1536:191439360:17050875
```

Thresholded Q4_K/Q6_K row-level Rayon scheduling was already tested and
rejected. The next optimization should change kernel arithmetic, activation
format, or storage layout rather than retrying the same scheduler shape.

## Current Ferrite Shape

Ferrite's aarch64 Q4_K and Q6_K kernels currently keep the right-hand activation
vector as `f32`.

- `crates/ferrite-inference/src/scalar/q4_k_neon.rs`
  - `neon_q4_k_block_dot` unpacks Q4_K nibbles into short `[f32; 4]`
    temporaries.
  - It loads those temporaries with `vld1q_f32`, applies `d` and `dmin`, then
    performs `vfmaq_f32` against the `f32` activation vector.
- `crates/ferrite-inference/src/scalar/q6_k_neon.rs`
  - `neon_q6_k_block_dot` similarly reconstructs four `[f32; 4]` quant lanes
    from low and high bits.
  - It multiplies by the block scale in `f32` before the fused multiply-add.

This shape is simple and correctness-friendly, but it spends hot-loop work on
scalar unpack, stack temporary fills, integer-to-float conversion, and `f32`
FMAs for values that are originally low-bit integer quants.

## llama.cpp Contrast

The relevant llama.cpp reference paths use a different contract:

- GGUF Q4_K and Q6_K weights are dotted against intermediate `block_q8_K`
  activation blocks, not directly against `f32` activations.
- `block_q8_K` stores an activation scale, 256 signed int8 quants, and 16 sums.
- Generic Q4_K/Q6_K dot paths unpack the weight quants to integer lanes, multiply
  by `q8_K` activation bytes, accumulate integer sums, then apply the combined
  scales.
- ARM NEON paths use int8 dot-style operations over these integer lanes.
- ARM repack paths add interleaved Q4_Kx8/Q4_Kx16 and Q6_Kx8 layouts for groups
  of rows, which is another larger storage-layout change beyond Ferrite's
  current direct GGUF block layout.

Directly copying the fast llama.cpp ARM kernel is therefore not a local edit.
Ferrite would first need an activation-side `q8_K` representation, and possibly
an interleaved row layout if the goal is to match the highest-throughput
reference strategy.

## Candidate Paths

### Path A: Local F32 Kernel Cleanup

Keep the public `f32` activation contract and reduce the scalar unpack overhead
inside `neon_q4_k_block_dot` and `neon_q6_k_block_dot`.

Possible work:

- unpack larger byte groups with NEON integer operations;
- convert integer lanes to `f32` vectors without stack `[f32; 4]` temporaries;
- keep the decoded value computation in vector registers before `vfmaq_f32`.

Pros:

- Smallest code change.
- Fits current tests and matrix dispatch.
- No model loader or activation representation changes.

Risks:

- It still pays `f32` activation bandwidth and `f32` arithmetic cost.
- It cannot use the core `q4_K/q6_K x q8_K` dot strategy from llama.cpp.
- It may not move Qwen2.5-1.5B enough because the operation remains structurally
  different from the proven fast reference path.

### Path B: Add Activation `q8_K` Dot Path

Introduce a focused internal activation quantization path for hot quantized
matvecs:

- define a small `q8_k` module with a `BlockQ8K` representation;
- quantize each 256-value activation segment once per matvec input vector;
- add Q4_K x Q8_K and Q6_K x Q8_K dot helpers;
- route only eligible Q4_K/Q6_K matrix-vector calls through the new path after
  correctness gates pass.

Pros:

- Aligns with llama.cpp's Q4_K/Q6_K kernel contract.
- Moves hot arithmetic from repeated `f32` quant reconstruction into integer dot
  accumulation with combined scales.
- Can stay modular: `q8_k.rs`, `q4_k_q8_k_neon.rs`, and `q6_k_q8_k_neon.rs`
  rather than expanding existing files into large catch-all modules.

Risks:

- Larger semantic change than local cleanup.
- Activation quantization overhead must be amortized across enough rows.
- Needs careful tolerance design because `q8_K` activation quantization changes
  arithmetic from the current decode-to-f32 reference path.

### Path C: Add Repacked Row Layouts

Add interleaved Q4_K/Q6_K row groups, similar to llama.cpp's Q4_Kx8/Q4_Kx16 and
Q6_Kx8 structures.

Pros:

- Potentially better memory access and row-grouped SIMD.
- Compatible with repeated decode once weights are loaded.

Risks:

- Highest loader and storage complexity.
- More memory accounting impact.
- Should not happen before a `q8_K` activation dot path proves that the
  arithmetic contract is worth optimizing further.

## Recommended Next Slice

Prefer Path B as the next design slice, not another scheduler experiment.

The first implementation should remain narrow:

1. Add `BlockQ8K` and scalar activation quantization tests.
2. Add scalar Q4_K x Q8_K and Q6_K x Q8_K reference-dot tests against a
   deterministic fixture.
3. Add aarch64 NEON dot helpers only after scalar tests are red/green.
4. Gate the optimized route behind existing matrix eligibility checks.
5. Benchmark Qwen2.5-1.5B and SmolLM2-1.7B before claiming throughput progress.

Do not start with repacked weights. Repacking is a second-order optimization
after the `q8_K` activation contract proves correctness and benchmark value.

## Verification Gates For The Next Code Slice

Minimum focused gates:

```sh
cargo test -p ferrite-inference q8_k -- --nocapture
cargo test -p ferrite-inference q4_k_q8_k -- --nocapture
cargo test -p ferrite-inference q6_k_q8_k -- --nocapture
cargo test -p ferrite-inference --test matvec_kernel_check -- --nocapture
```

Minimum workspace gates before committing implementation:

```sh
cargo fmt --all -- --check
git diff --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p ferrite-inference --target x86_64-unknown-linux-gnu --tests
```

Minimum model gates before a throughput claim:

```sh
target/release/ferrite --model target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf --prompt 'hello world' --generate-tokens 3 --expect-generated-token-ids 198,9707,11
target/release/ferrite --model target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf --prompt 'The capital of France is' --generate-tokens 3 --expect-generated-token-ids 12095,13,576
/usr/bin/time -l target/release/ferrite --model target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf --prompt 'hello world' --benchmark-runs 3 --profile-benchmark-token
target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt 'hello world' --generate-tokens 6 --expect-generated-token-ids 18,198,3725,198,198,788
target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt 'The capital of France is' --generate-tokens 6 --expect-generated-token-ids 7042,30,2
```

## Decision

Record this as the next kernel hypothesis. Implementation should begin only as a
small TDD slice with dedicated modules for `q8_K` activation blocks and the
Q4_K/Q6_K dot adapters, keeping existing kernel files focused.
