# 2026-07-06 Locus KV Backend

## Purpose

Record evidence for the optional Locus KV-block backend (`KvCacheStore::Locus`,
gated behind the `locus-kv` cargo feature and `--kv-backend locus`; `Vec`
remains the default). This note separates what is **proven now**, without a
Tier 1 model artifact on this host, from what is **pending** a real-model run.

No real Tier 1 GGUF (e.g. Qwen2.5-1.5B) is present on this machine
(`target/models/` does not exist; no `.gguf` file is tracked or cached in the
repo tree), and per Ferrite's operating model this task did not download one.
Every number below is either a pasted real command output or explicitly
labeled PENDING, nothing is estimated or invented.

## Tree State

- Branch: `feat/locus-kv-backend`
- Commit: `ca89d254864e611b03d46ad63d4a0fe49b90cc9e` (`git rev-parse HEAD`)
- Working tree before this note: clean

## Hardware and OS

```sh
uname -a
```

```text
Darwin Victors-MBP-2.localdomain 25.5.0 Darwin Kernel Version 25.5.0: Tue Jun  9 22:28:34 PDT 2026; root:xnu-12377.121.10~1/RELEASE_ARM64_T6050 arm64
```

- Machine: MacBook Pro, `Mac17,9`, Apple M5 Pro (`sysctl -n hw.model`,
  `system_profiler SPHardwareDataType`)
- Logical CPUs: 15 (5 Super + 10 Performance)
- RAM: 24 GB
- rustc 1.96.0 / cargo 1.96.0 (`rustc --version`, `cargo --version`)

## Model

No Tier 1 model was available or downloaded for this note. See PENDING below
for the exact repro commands against `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
(the artifact already referenced by the README and other benchmark notes in
this repo). The optional end-to-end CLI check in PROVEN step 4 uses
`ferrite_fixtures::scalar_llama_f32_gguf_fixture()`, a tiny synthetic
single-layer GGUF used elsewhere in this repo's test suite, it proves
plumbing and allocation mechanics only, **not** throughput or memory behavior.

## Build mode

Release, `-p ferrite-cli --features locus-kv` (see PROVEN step 1).

---

## PROVEN

### 1. Release build with the feature

```sh
cargo build --release -p ferrite-cli --features locus-kv
```

```text
    Finished `release` profile [optimized] target(s) in 2.73s
```

Build succeeds; the `locus-kv` feature compiles cleanly into the release CLI
binary.

### 2. Correctness parity (real forward path, differential test)

```sh
cargo test -p ferrite-inference --features locus-kv --test kv_store_backend_parity -- --nocapture
```

```text
running 1 test
test locus_backend_matches_vec_logits ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`crates/ferrite-inference/tests/kv_store_backend_parity.rs` loads a fixture
GGUF with non-trivial (identity-mixing) Q/K/V/O and FFN weights, so cached
keys/values genuinely mix across positions, unlike the quantized fixtures
whose attention/FFN weights are zero. It runs `accept_prompt` on the same
5-token prompt (spanning a 4-token block boundary) through the Vec backend and
the Locus backend (`tokens_per_block: 4, max_tokens: 64`) and asserts
bit-identical `token_id`, `logits`, and `kv_cache_bytes`.

### 3. Allocation mechanics (LocusKvStore unit tests)

```sh
cargo test -p ferrite-inference --features locus-kv --lib scalar::kv_store::locus -- --nocapture
```

```text
running 4 tests
test scalar::kv_store::locus::tests::locus_store_round_trips_across_block_boundaries ... ok
test scalar::kv_store::locus::tests::locus_store_reports_out_of_blocks ... ok
test scalar::kv_store::locus::tests::locus_store_truncates_and_frees_blocks ... ok
test scalar::kv_store::locus::tests::locus_store_snapshot_round_trip ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 95 filtered out; finished in 0.00s
```

These confirm the pool mechanics directly: values round-trip correctly across
block boundaries; `truncate` frees blocks back to the pool
(`pool_stats().allocated` strictly decreases after truncation); pushing past
pool capacity produces an explicit out-of-blocks error rather than silent
corruption.

### 4. End-to-end CLI plumbing on a synthetic fixture GGUF (optional step, achieved)

A throwaway `#[test]` was added to `crates/ferrite-fixtures/tests/` to write
`scalar_llama_f32_gguf_fixture()` bytes to a scratchpad path, run once, then
deleted immediately (`git status --porcelain` was clean before and after; no
test churn was committed). The fixture is a single-layer (`llama.block_count:
1`) synthetic model, so this proves CLI + Locus plumbing only.

```sh
target/release/ferrite --model <scratchpad>/fixture.gguf --prompt-token-ids 0,1,2,0,1 --generate-tokens 3
```

```text
prompt_token_ids=0,1,2,0,1
experimental_q8_k_activation_matvec=false
compare_q8_k_activation_matvec=false
q8_k_activation_matvec_policy=default_only
q8_k_activation_matvec_roles=all
next_token_id=2
next_token=winner
generated_cached_tokens=8
generated_token_ids=2,2,2
generated_stopped_on_eos=false
generated_text=winnerwinnerwinner
model_file_bytes=2064
model_file_retained_bytes=0
scalar_weight_bytes=184
kv_cache_bytes=128
```

```sh
target/release/ferrite --model <scratchpad>/fixture.gguf --prompt-token-ids 0,1,2,0,1 --generate-tokens 3 --kv-backend locus --kv-tokens-per-block 2
```

```text
prompt_token_ids=0,1,2,0,1
experimental_q8_k_activation_matvec=false
compare_q8_k_activation_matvec=false
q8_k_activation_matvec_policy=default_only
q8_k_activation_matvec_roles=all
next_token_id=2
next_token=winner
generated_cached_tokens=8
generated_token_ids=2,2,2
generated_stopped_on_eos=false
generated_text=winnerwinnerwinner
model_file_bytes=2064
model_file_retained_bytes=0
scalar_weight_bytes=184
kv_cache_bytes=128
locus_pool_allocation_count=8
```

Vec and Locus backends produce identical `next_token_id`, `generated_token_ids`,
and `kv_cache_bytes` end to end through the CLI, matching the library-level
parity test. `locus_pool_allocation_count=8` is real, measured output: this
fixture has 1 layer, `tokens_per_block=2`, and 8 total cached tokens (5 prompt
+ 3 generated), and `2 * num_layers * ceil(tokens/tokens_per_block) = 2 * 1 *
ceil(8/2) = 8`, the analytic block-allocation formula (see below) matches the
observed counter exactly.

**This step does not demonstrate throughput or memory behavior**, the model
is a few-KB synthetic fixture with a single layer; its timing and RSS are not
representative of anything.

---

## Analytic allocation argument (analysis, not a measured throughput/RSS result)

This is a structural argument about allocation counts, stated explicitly as
analysis:

- The Vec backend heap-allocates 2 `Vec<f32>` per layer per accepted token:
  `2 * num_layers * tokens` small heap allocations over a sequence, with no
  pooling or reuse (`VecKvStore::push` calls `key.to_vec()` /
  `value.to_vec()` unconditionally). There is no counter for this in the Vec
  path; the figure comes directly from reading `push` in
  `crates/ferrite-inference/src/scalar/kv_store.rs`.
- The Locus backend allocates one pool block per `tokens_per_block` tokens per
  (layer, K/V) list: `2 * num_layers * ceil(tokens / tokens_per_block)` block
  allocations over a sequence of that many cached tokens
  (`crates/ferrite-inference/src/scalar/kv_store/locus.rs`), a
  ~`tokens_per_block`× reduction in KV allocation count within the session
  versus the Vec backend's one allocation per token.
- The pool is per-session: `LocusKvStore::new` creates its own
  `KvBlockPool`, owned by the `ScalarLlamaSession` built in
  `start_session_with_options`, and the whole pool is released when the
  session drops. `KvReuseOrder::Lifo` reuse (`KvBlockPool::free`) therefore
  benefits only within-session reallocation, blocks freed by `truncate`
  (or `restore`) are handed back out warm before any fresh block is
  allocated. There is no cross-session block reuse; that would require a
  longer-lived shared pool owned above the session, which is not part of
  this design.
- With `tokens_per_block = 16` (the CLI default), the allocation-count ratio
  approaches `tokens_per_block` (≈16×) as `tokens` grows, independent of
  `num_layers` (it cancels in the ratio). This was directly confirmed on the
  synthetic fixture above (`tokens_per_block = 2` → 8 measured block
  allocations vs. an unmeasured 16 Vec-backend allocations by the same
  formula).
- This is an **allocation-count** argument only, and only about
  within-session reallocation. It says nothing about measured RSS or
  throughput on a real model, and nothing about cross-session behavior,   those require PENDING work below.

---

## PENDING (requires a real Tier 1 model artifact + explicit download authorization)

None of the following have been run. Each command below mirrors the existing
README CLI/server forms with the Locus flags added
(`--kv-backend locus --kv-tokens-per-block 16 --kv-max-tokens <cap>`).

### Correctness parity on a real model

```sh
target/release/ferrite --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf --prompt 'hello world' --generate-tokens 1
target/release/ferrite --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf --prompt 'hello world' --generate-tokens 1 --kv-backend locus --kv-tokens-per-block 16 --kv-max-tokens 256
```

Expected: identical generated token id (same gate as the fixture-level
parity test above, but on real dequantized weights).

### Allocation churn on a real model

```sh
target/release/ferrite --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf --prompt 'hello world' --generate-tokens 64 --kv-backend locus --kv-tokens-per-block 16 --kv-max-tokens 256
```

Record `locus_pool_allocation_count` from stdout and confirm it equals
`2 * num_layers * ceil(tokens / 16)` for the real model's layer count.

### Memory (RSS)

Using the documented "CLI Memory Sampling" flow (`README.md`), for both
backends, at a short (32-token) and a longer generation:

```sh
target/release/ferrite --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf --prompt 'hello world' --sleep-after-load-ms 5000 --generate-tokens 32
target/release/ferrite --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf --prompt 'hello world' --sleep-after-load-ms 5000 --generate-tokens 32 --kv-backend locus --kv-tokens-per-block 16 --kv-max-tokens 256
```

Peak RSS via `/usr/bin/time -l` for both backends and both lengths, e.g.:

```sh
/usr/bin/time -l target/release/ferrite --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf --prompt 'hello world' --generate-tokens 512
/usr/bin/time -l target/release/ferrite --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf --prompt 'hello world' --generate-tokens 512 --kv-backend locus --kv-tokens-per-block 16 --kv-max-tokens 1024
```

Gate (per the design spec's success gate): Locus RSS ≤ Vec RSS at short
sequence length (no regression), ideally lower at longer lengths.

### Throughput

```sh
target/release/ferrite --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf --prompt 'hello world' --benchmark-runs 20
target/release/ferrite --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf --prompt 'hello world' --benchmark-runs 20 --kv-backend locus --kv-tokens-per-block 16 --kv-max-tokens 256
```

Gate: Locus tok/s within measurement noise of Vec (no loss). Record
`benchmark_total_ns` / `benchmark_avg_ns` for both.

### Server opt-in path

The `ferrite-server` crate currently has no `locus-kv` cargo feature and no
`--kv-backend`/`KvBackend` reference anywhere in `crates/ferrite-server/src`
(confirmed by `grep -rln "kv-backend\|KvBackend\|locus-kv" crates/ferrite-server/`,
which returns nothing, and by `crates/ferrite-server/Cargo.toml`, which has no
`[features]` entry for `locus-kv`). The server-side opt-in path is not merely
unmeasured, it does not exist yet. Wiring it is future work, not part of this
evidence task.

### x86_64 and NUMA

Not measured; this host is Apple Silicon (arm64) only. Per the design spec,
NUMA binding is explicitly out of scope for this slice (Locus's `numa`
feature is Linux-only and not enabled).

## Interpretation

The Locus KV backend is proven correct (bit-identical logits/tokens/kv byte
accounting vs. the Vec backend) on both a differential fixture test and an
end-to-end CLI run, and its pool mechanics (allocate-on-block-boundary,
free-on-truncate, LIFO reuse, out-of-blocks error) are proven by unit tests
and by a real measured allocation count on the CLI fixture run matching the
analytic formula exactly. The feature builds cleanly into the release CLI.

What remains unproven is exactly the numbers the design's success gate is
about: real-model RSS and throughput. Those require a Tier 1 GGUF artifact
this task was not authorized to download, and are listed above as PENDING
with exact repro commands so a future session (or a human with the model
already present) can fill them in without re-deriving the protocol.
