# Q8_K Activation Dot Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a tested internal `q8_K` activation-dot path for Q4_K and Q6_K matvecs, then prove whether it improves Tier 1 CPU decode throughput.

**Architecture:** Keep public matvec APIs on `&[f32]`, quantize eligible 256-value activation chunks into internal `BlockQ8K` blocks, and use format-specific Q4_K/Q6_K x Q8_K dot helpers behind the existing dispatchers. Scalar helpers define the arithmetic contract before aarch64 NEON helpers are added.

**Tech Stack:** Rust workspace, `ferrite-inference`, existing scalar quantized matrix modules, aarch64 NEON intrinsics, Cargo tests, Ferrite CLI model-output checks.

---

## File Structure

- Create `crates/ferrite-inference/src/scalar/q8_k.rs`
  - Owns `BlockQ8K`, activation quantization, and validation for 256-value activation blocks.
- Create `crates/ferrite-inference/src/scalar/q4_k_q8_k.rs`
  - Owns scalar Q4_K x Q8_K dot helpers and row matvec adapter.
- Create `crates/ferrite-inference/src/scalar/q6_k_q8_k.rs`
  - Owns scalar Q6_K x Q8_K dot helpers and row matvec adapter.
- Create `crates/ferrite-inference/src/scalar/q4_k_q8_k_neon.rs`
  - Owns aarch64 NEON Q4_K x Q8_K helpers after scalar tests pass.
- Create `crates/ferrite-inference/src/scalar/q6_k_q8_k_neon.rs`
  - Owns aarch64 NEON Q6_K x Q8_K helpers after scalar tests pass.
- Modify `crates/ferrite-inference/src/scalar.rs`
  - Registers the focused modules.
- Modify `crates/ferrite-inference/src/scalar/q4_k.rs`
  - Routes eligible Q4_K matvec calls through the new adapter after tests pass.
- Modify `crates/ferrite-inference/src/scalar/q6_k.rs`
  - Routes eligible Q6_K matvec and argmax-sensitive calls deliberately; argmax can stay on the existing path until separately proven.
- Modify `documentation/dev-notes/`
  - Records each implementation and benchmark slice.

## Task 1: Q8_K Activation Blocks

**Files:**
- Create: `crates/ferrite-inference/src/scalar/q8_k.rs`
- Modify: `crates/ferrite-inference/src/scalar.rs`

- [ ] **Step 1: Write failing tests**

Add unit tests proving that a deterministic 256-value vector quantizes into a
finite scale, int8 values in range, and 16 group sums.

- [ ] **Step 2: Run red test**

Run:

```sh
cargo test -p ferrite-inference q8_k -- --nocapture
```

Expected: fail because `q8_k` does not exist or `BlockQ8K` is missing.

- [ ] **Step 3: Implement minimal `BlockQ8K`**

Implement only:

- `BlockQ8K { d: f32, qs: [i8; 256], bsums: [i16; 16] }`
- `BlockQ8K::quantize(values: &[f32]) -> Result<Self, InferenceError>`
- validation that the input length is exactly 256.

- [ ] **Step 4: Run green test**

Run:

```sh
cargo test -p ferrite-inference q8_k -- --nocapture
```

Expected: pass.

- [ ] **Step 5: Commit**

Commit message:

```text
feat: add q8 k activation blocks
```

## Task 2: Scalar Q4_K x Q8_K Contract

**Files:**
- Create: `crates/ferrite-inference/src/scalar/q4_k_q8_k.rs`
- Modify: `crates/ferrite-inference/src/scalar.rs`
- Modify: `crates/ferrite-inference/src/scalar/q4_k.rs` only if a small helper needs visibility.

- [ ] **Step 1: Write failing tests**

Add tests comparing scalar Q4_K x Q8_K dot output against existing
`q4_k_block_values(block)` dotted with the original `f32` vector. Use an
explicit tolerance because activation quantization changes arithmetic.

- [ ] **Step 2: Run red test**

Run:

```sh
cargo test -p ferrite-inference q4_k_q8_k -- --nocapture
```

Expected: fail because the new helper does not exist.

- [ ] **Step 3: Implement scalar helper**

Implement:

- `q4_k_q8_k_block_dot(block: &[u8], activation: &BlockQ8K) -> Result<f32, InferenceError>`
- `q4_k_q8_k_mul_vec(bytes, rows, cols, vector) -> Result<Vec<f32>, InferenceError>` if block tests pass.

- [ ] **Step 4: Run green tests**

Run:

```sh
cargo test -p ferrite-inference q4_k_q8_k -- --nocapture
cargo test -p ferrite-inference --test matvec_kernel_check -- --nocapture
```

Expected: pass.

- [ ] **Step 5: Commit**

Commit message:

```text
feat: add scalar q4 k q8 k dot
```

## Task 3: Scalar Q6_K x Q8_K Contract

**Files:**
- Create: `crates/ferrite-inference/src/scalar/q6_k_q8_k.rs`
- Modify: `crates/ferrite-inference/src/scalar.rs`
- Modify: `crates/ferrite-inference/src/scalar/q6_k.rs` only if a small helper needs visibility.

- [ ] **Step 1: Write failing tests**

Add tests comparing scalar Q6_K x Q8_K dot output against existing
`q6_k_block_values(block)` dotted with the original `f32` vector.

- [ ] **Step 2: Run red test**

Run:

```sh
cargo test -p ferrite-inference q6_k_q8_k -- --nocapture
```

Expected: fail because the new helper does not exist.

- [ ] **Step 3: Implement scalar helper**

Implement:

- `q6_k_q8_k_block_dot(block: &[u8], activation: &BlockQ8K) -> Result<f32, InferenceError>`
- row matvec support only after block-dot tests pass.

- [ ] **Step 4: Run green tests**

Run:

```sh
cargo test -p ferrite-inference q6_k_q8_k -- --nocapture
cargo test -p ferrite-inference --test matvec_kernel_check -- --nocapture
```

Expected: pass.

- [ ] **Step 5: Commit**

Commit message:

```text
feat: add scalar q6 k q8 k dot
```

## Task 4: Dispatcher Route And Model Parity

**Files:**
- Modify: `crates/ferrite-inference/src/scalar/q4_k.rs`
- Modify: `crates/ferrite-inference/src/scalar/q6_k.rs`
- Add: `documentation/dev-notes/2026-06-28-q8-k-dispatch-slice.md`

- [ ] **Step 1: Write failing routing tests**

Add tests proving eligible Q4_K/Q6_K shapes use the new adapter while invalid
or unsupported shapes keep existing fallback behavior.

- [ ] **Step 2: Run red tests**

Run:

```sh
cargo test -p ferrite-inference q8_k_dispatch -- --nocapture
```

Expected: fail because dispatch is not wired.

- [ ] **Step 3: Wire conservative dispatch**

Route only shapes where:

- `cols != 0`;
- `cols.is_multiple_of(256)`;
- input vector length equals `cols`;
- scalar Q8_K adapter tests already pass.

Leave Q6_K argmax on its existing path unless a focused argmax test proves the
new route.

- [ ] **Step 4: Run focused and parity tests**

Run:

```sh
cargo test -p ferrite-inference q8_k_dispatch -- --nocapture
cargo test -p ferrite-inference --test matvec_kernel_check -- --nocapture
```

Expected: pass.

- [ ] **Step 5: Run model parity checks**

Run the established Qwen2.5-1.5B and SmolLM2-1.7B prompt checks from
`documentation/research/2026-06-28-tier1-q4-q6-kernel-hypothesis.md`.

- [ ] **Step 6: Commit**

Commit message:

```text
feat: route q4 q6 through q8 k activations
```

## Task 5: AArch64 NEON Q4_K/Q6_K x Q8_K Helpers

**Files:**
- Create: `crates/ferrite-inference/src/scalar/q4_k_q8_k_neon.rs`
- Create: `crates/ferrite-inference/src/scalar/q6_k_q8_k_neon.rs`
- Modify: `crates/ferrite-inference/src/scalar.rs`
- Modify: `crates/ferrite-inference/src/scalar/q4_k_q8_k.rs`
- Modify: `crates/ferrite-inference/src/scalar/q6_k_q8_k.rs`

- [ ] **Step 1: Write failing NEON comparison tests**

Add aarch64-only tests that compare NEON Q4_K/Q6_K x Q8_K block dots against the
scalar Q8_K helpers.

- [ ] **Step 2: Run red tests**

Run:

```sh
cargo test -p ferrite-inference q8_k_neon -- --nocapture
```

Expected: fail because NEON helpers do not exist.

- [ ] **Step 3: Implement NEON helpers**

Use safe dispatchers, target-feature checks, documented unsafe blocks, and the
existing SIMD unsafe-boundary conventions.

- [ ] **Step 4: Run green tests**

Run:

```sh
cargo test -p ferrite-inference q8_k_neon -- --nocapture
cargo test -p ferrite-inference --test matvec_kernel_check -- --nocapture
```

Expected: pass.

- [ ] **Step 5: Commit**

Commit message:

```text
feat: add neon q4 q6 q8 k dots
```

## Task 6: Full Gates And Benchmark Evidence

**Files:**
- Add: `documentation/benchmarks/2026-06-28-tier1-q8-k-activation-dot.md`
- Modify: `documentation/engineering/tier1-gate-status.md`

- [ ] **Step 1: Run workspace gates**

Run:

```sh
cargo fmt --all -- --check
git diff --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p ferrite-inference --target x86_64-unknown-linux-gnu --tests
```

- [ ] **Step 2: Run model parity and benchmark commands**

Run the Qwen2.5-1.5B and SmolLM2-1.7B parity commands plus benchmark-token
profile commands recorded in the research note.

- [ ] **Step 3: Record benchmark note and Tier 1 status update**

Document commit, hardware, command lines, model artifacts, prompt, thread count,
RSS, timings, and whether throughput improved or regressed.

- [ ] **Step 4: Commit**

Commit message:

```text
docs: record q8 k activation dot evidence
```

## Self-Review

- The plan implements ADR 0007's approved Path B only.
- It does not include repacked row layouts.
- It keeps `q8_K`, Q4_K adapter, Q6_K adapter, and NEON helpers in focused
  modules.
- It preserves scalar/reference gates before optimized dispatch.
- It leaves AVX2 runtime optimization for a separate future slice.
