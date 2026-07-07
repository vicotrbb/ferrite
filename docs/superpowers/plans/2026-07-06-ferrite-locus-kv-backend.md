# Ferrite Locus KV-Block Backend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an opt-in, single-owner KV-cache storage backend built on Locus's `KvBlockPool`, behind a new `KvCacheStore` seam, to cut Ferrite's per-token allocation churn while keeping the proven `Vec<Vec<Vec<f32>>>` path as the default with identical output.

**Architecture:** A `KvCacheStore` enum sits between `ScalarLlamaSession` and its KV storage. `VecKvStore` reproduces today's behavior exactly and is the default. `LocusKvStore` (compiled only under the `locus-kv` Cargo feature) stores K/V in fixed-size, mapped, page-aligned Locus blocks allocated incrementally at `tokens_per_block` granularity, read zero-copy via `bytemuck`. The prefix-cache snapshot format (`ScalarLlamaSessionSnapshot`, owned nested `Vec`s) is unchanged; stores marshal to/from it by copy.

**Tech Stack:** Rust 2021, `locus-alloc` (path dep), `bytemuck` (safe zero-copy `&[u8]`↔`&[f32]`), `rayon` (existing), `ferrite-fixtures` (deterministic test model).

## Global Constraints

- Workspace clippy lints DENY `unwrap_used`, `expect_used`, `panic`, and rustc denies `unsafe_code` — in ALL targets including tests. Use `?`, `Result`, `match`, and checked access; no `.unwrap()`, `.expect()`, `panic!`, `unsafe` anywhere. Tests return `Result<(), InferenceError>` (or a suitable error) and use `?`.
- `unsafe` is forbidden in Ferrite crates. All byte↔float reinterpretation goes through `bytemuck`'s safe API (`try_cast_slice` / `try_cast_slice_mut`); the `unsafe` lives inside `bytemuck`.
- Default behavior must not change: with the `locus-kv` feature off and `KvBackend::Vec` (the default), output must be byte-for-byte identical to today. `start_session()` stays infallible.
- Correctness is a hard gate: the Locus backend must produce bit-identical KV and logits to the Vec backend (differential test + fixture-model parity test), and must keep matching the existing scalar reference tests.
- Locus is consumed as-is via a path dependency: `locus-alloc = { path = "../locus/crates/locus-alloc" }` relative to the `ferrite` repo root's sibling `locus` checkout. Do NOT enable the `numa` feature (Linux-only). Do NOT modify Locus.
- Evidence discipline (Ferrite operating model): land a dev-note, a `documentation/benchmarks/` note, and an ADR. No performance/memory claim without measured evidence (host, model, quant, prompt, token counts, thread count, build mode, commit/tree state).
- KV vector length per token per K or V = `config.attention_head_count_kv * config.head_dim` (call it `head_kv_dim`). This equals `expected_kv` in `attention.rs`.

## File Structure

- Create `crates/ferrite-inference/src/scalar/kv_store.rs` — the `KvCacheStore` enum, `VecKvStore`, shared errors, and the store method surface. One responsibility: KV storage behind a stable interface.
- Create `crates/ferrite-inference/src/scalar/kv_store/locus.rs` — `LocusKvStore` (only under `feature = "locus-kv"`).
- Modify `crates/ferrite-inference/src/scalar/options.rs` — add `KvBackend` and a field on `ScalarExecutionOptions`.
- Modify `crates/ferrite-inference/src/scalar.rs` — declare `mod kv_store;`, re-export, make `start_session_with_options` fallible.
- Modify `crates/ferrite-inference/src/scalar/session.rs` — replace the two `Vec<Vec<Vec<f32>>>` fields with one `store: KvCacheStore`; route push/read/count through it.
- Modify `crates/ferrite-inference/src/scalar/attention.rs` — read KV through the store instead of `&[Vec<f32>]`.
- Modify `crates/ferrite-inference/src/scalar/session/cache.rs` — `new`, `new_with_options`, `truncate_cache`, `kv_cache_bytes` via the store.
- Modify `crates/ferrite-inference/src/scalar/session/snapshot.rs` — `cache_snapshot`/`restore_cache_snapshot` via the store; add `pub(in crate::scalar)` snapshot constructor + accessors.
- Modify `crates/ferrite-inference/Cargo.toml` — optional `locus-alloc` + `bytemuck` deps, `locus-kv` feature.
- Modify `crates/ferrite-cli/src/args.rs`, `run.rs`, `benchmark.rs` — `--kv-backend` / `--kv-tokens-per-block` / `--kv-max-tokens`; handle the fallible session constructor.
- Create `crates/ferrite-inference/tests/kv_store_backend_parity.rs` — differential (Vec vs Locus) + fixture-model parity tests.
- Create docs: `documentation/dev-notes/2026-07-06-locus-kv-backend.md`, `documentation/benchmarks/2026-07-06-locus-kv-backend.md`, `documentation/adr/0010-locus-kv-block-backend.md`.

## Store interface (defined once; referenced by later tasks)

`KvCacheStore` is an enum with these methods (exact signatures — later tasks depend on them):

```rust
pub(in crate::scalar) fn layer_count(&self) -> usize;
pub(in crate::scalar) fn layer_len(&self, layer: usize) -> usize;
pub(in crate::scalar) fn push(&mut self, layer: usize, key: &[f32], value: &[f32]) -> Result<(), InferenceError>;
pub(in crate::scalar) fn key(&mut self, layer: usize, position: usize) -> Result<&[f32], InferenceError>;
pub(in crate::scalar) fn value(&mut self, layer: usize, position: usize) -> Result<&[f32], InferenceError>;
pub(in crate::scalar) fn truncate(&mut self, token_count: usize) -> Result<(), InferenceError>;
pub(in crate::scalar) fn kv_cache_bytes(&self) -> u128;
pub(in crate::scalar) fn snapshot(&mut self, cached_token_count: usize) -> Result<ScalarLlamaSessionSnapshot, InferenceError>;
pub(in crate::scalar) fn restore(&mut self, snapshot: &ScalarLlamaSessionSnapshot) -> Result<(), InferenceError>;
```

Semantics:
- `push(layer, key, value)` appends one position to `layer`; the position is `layer_len(layer)` before the call. Every layer is pushed once per accepted token, so within a token the current layer transiently has one more position than not-yet-pushed layers.
- `key`/`value` return the exact `head_kv_dim`-length slice for `(layer, position)`. They take `&mut self` (Locus requires it); the Vec backend ignores the mutability.
- `kv_cache_bytes()` returns logical `f32` bytes: `sum over layers of layer_len * head_kv_dim * 4 * 2`. Identical across backends so the prefix-cache byte budget is unaffected.
- `snapshot` materializes an owned `ScalarLlamaSessionSnapshot`; `restore` rebuilds storage from one.

---

### Task 1: Add `KvBackend` selection to `ScalarExecutionOptions`

**Files:**
- Modify: `crates/ferrite-inference/src/scalar/options.rs`

**Interfaces:**
- Produces: `pub enum KvBackend { Vec, Locus { tokens_per_block: usize, max_tokens: usize } }` (derives `Clone, Copy, Debug, PartialEq, Eq`); `ScalarExecutionOptions::with_kv_backend(self, KvBackend) -> Self`; `ScalarExecutionOptions::kv_backend(self) -> KvBackend`. Default is `KvBackend::Vec`.

- [ ] **Step 1: Write the failing test**

Add to the `tests` module at the bottom of `crates/ferrite-inference/src/scalar/options.rs`:

```rust
    #[test]
    fn default_kv_backend_is_vec() {
        let options = ScalarExecutionOptions::default();
        assert_eq!(options.kv_backend(), super::KvBackend::Vec);
    }

    #[test]
    fn with_kv_backend_selects_locus() {
        let backend = super::KvBackend::Locus {
            tokens_per_block: 16,
            max_tokens: 256,
        };
        let options = ScalarExecutionOptions::default().with_kv_backend(backend);
        assert_eq!(options.kv_backend(), backend);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p ferrite-inference --lib scalar::options`
Expected: FAIL — `KvBackend` / `with_kv_backend` / `kv_backend` not found.

- [ ] **Step 3: Write minimal implementation**

At the top of `options.rs` (after the existing `use`-free header), add the enum:

```rust
/// Selects the KV-cache storage backend for a scalar session.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KvBackend {
    /// Default nested-`Vec` storage (today's behavior).
    #[default]
    Vec,
    /// Locus block-pool storage (requires the `locus-kv` feature at build time).
    Locus {
        /// Tokens stored per fixed-size block.
        tokens_per_block: usize,
        /// Maximum tokens the pool is sized for; exceeding it is an error.
        max_tokens: usize,
    },
}
```

Add a field to `ScalarExecutionOptions` (it stays `Copy` — `KvBackend` is `Copy`):

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScalarExecutionOptions {
    q8_k_activation_matvec_policy: Q8KActivationMatvecPolicy,
    q8_k_activation_matvec_roles: Q8KActivationMatvecRoleMask,
    compare_q8_k_activation_matvec: bool,
    kv_backend: KvBackend,
}
```

Add the accessors inside `impl ScalarExecutionOptions`:

```rust
    pub fn with_kv_backend(mut self, backend: KvBackend) -> Self {
        self.kv_backend = backend;
        self
    }

    pub fn kv_backend(self) -> KvBackend {
        self.kv_backend
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p ferrite-inference --lib scalar::options`
Expected: PASS.

- [ ] **Step 5: Re-export `KvBackend`**

In `crates/ferrite-inference/src/scalar.rs`, extend the existing options re-export (line ~57) to include `KvBackend`:

```rust
pub use options::{
    KvBackend, Q8KActivationMatvecPolicy, Q8KActivationMatvecRole, ScalarExecutionOptions,
};
```

Run: `cargo build -p ferrite-inference`
Expected: builds.

- [ ] **Step 6: Commit**

```bash
git add crates/ferrite-inference/src/scalar/options.rs crates/ferrite-inference/src/scalar.rs
git commit -m "feat: add KvBackend selection to scalar execution options"
```

---

### Task 2: `KvCacheStore` enum + `VecKvStore` backend

**Files:**
- Create: `crates/ferrite-inference/src/scalar/kv_store.rs`
- Modify: `crates/ferrite-inference/src/scalar.rs` (add `mod kv_store;`)

**Interfaces:**
- Consumes: `ScalarLlamaSessionSnapshot` (from `session::snapshot`), `InferenceError`.
- Produces: `KvCacheStore` with the method surface in "Store interface" above, plus `KvCacheStore::new_vec(layer_count: usize, head_kv_dim: usize) -> Self`. Needs snapshot marshalling helpers added in Task 5's snapshot edits — but for this task the Vec backend builds/reads the snapshot's public-in-crate constructor. Add that constructor now (see Step 3b).

- [ ] **Step 1: Write the failing test**

Create `crates/ferrite-inference/src/scalar/kv_store.rs` with only a `tests` module to start:

```rust
#[cfg(test)]
mod tests {
    use super::KvCacheStore;
    use crate::scalar::InferenceError;

    fn sample(layer: usize, position: usize, dim: usize) -> Vec<f32> {
        (0..dim)
            .map(|d| (layer * 1000 + position * 10 + d) as f32)
            .collect()
    }

    #[test]
    fn vec_store_round_trips_positions() -> Result<(), InferenceError> {
        let dim = 4;
        let mut store = KvCacheStore::new_vec(2, dim);
        for position in 0..3 {
            for layer in 0..2 {
                store.push(layer, &sample(layer, position, dim), &sample(layer + 100, position, dim))?;
            }
        }
        for layer in 0..2 {
            assert_eq!(store.layer_len(layer), 3);
            for position in 0..3 {
                assert_eq!(store.key(layer, position)?, sample(layer, position, dim).as_slice());
                assert_eq!(store.value(layer, position)?, sample(layer + 100, position, dim).as_slice());
            }
        }
        Ok(())
    }

    #[test]
    fn vec_store_truncates() -> Result<(), InferenceError> {
        let dim = 2;
        let mut store = KvCacheStore::new_vec(1, dim);
        for position in 0..4 {
            store.push(0, &sample(0, position, dim), &sample(0, position, dim))?;
        }
        store.truncate(2)?;
        assert_eq!(store.layer_len(0), 2);
        assert!(store.key(0, 2).is_err());
        Ok(())
    }

    #[test]
    fn vec_store_snapshot_round_trip() -> Result<(), InferenceError> {
        let dim = 3;
        let mut store = KvCacheStore::new_vec(2, dim);
        for position in 0..2 {
            for layer in 0..2 {
                store.push(layer, &sample(layer, position, dim), &sample(layer, position, dim))?;
            }
        }
        let snapshot = store.snapshot(2)?;
        let mut restored = KvCacheStore::new_vec(2, dim);
        restored.restore(&snapshot)?;
        for layer in 0..2 {
            assert_eq!(restored.layer_len(layer), 2);
            for position in 0..2 {
                assert_eq!(restored.key(layer, position)?, store.key(layer, position)?);
            }
        }
        Ok(())
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

First declare the module so it compiles. In `crates/ferrite-inference/src/scalar.rs`, add near the other `mod` lines (after `mod float;` grouping, alphabetical is fine):

```rust
mod kv_store;
```

Run: `cargo test -p ferrite-inference --lib scalar::kv_store`
Expected: FAIL — `KvCacheStore` / `new_vec` not found.

- [ ] **Step 3a: Write the `KvCacheStore` enum + `VecKvStore`**

At the top of `crates/ferrite-inference/src/scalar/kv_store.rs` (above the `tests` module):

```rust
use super::session::ScalarLlamaSessionSnapshot;
use super::InferenceError;

/// KV-cache storage behind a stable interface. `Vec` is the default backend and
/// reproduces the historical nested-`Vec` behavior exactly.
#[derive(Debug)]
pub(in crate::scalar) enum KvCacheStore {
    Vec(VecKvStore),
    #[cfg(feature = "locus-kv")]
    Locus(locus::LocusKvStore),
}

impl KvCacheStore {
    pub(in crate::scalar) fn new_vec(layer_count: usize, head_kv_dim: usize) -> Self {
        KvCacheStore::Vec(VecKvStore::new(layer_count, head_kv_dim))
    }

    pub(in crate::scalar) fn layer_count(&self) -> usize {
        match self {
            KvCacheStore::Vec(store) => store.layer_count(),
            #[cfg(feature = "locus-kv")]
            KvCacheStore::Locus(store) => store.layer_count(),
        }
    }

    pub(in crate::scalar) fn layer_len(&self, layer: usize) -> usize {
        match self {
            KvCacheStore::Vec(store) => store.layer_len(layer),
            #[cfg(feature = "locus-kv")]
            KvCacheStore::Locus(store) => store.layer_len(layer),
        }
    }

    pub(in crate::scalar) fn push(
        &mut self,
        layer: usize,
        key: &[f32],
        value: &[f32],
    ) -> Result<(), InferenceError> {
        match self {
            KvCacheStore::Vec(store) => store.push(layer, key, value),
            #[cfg(feature = "locus-kv")]
            KvCacheStore::Locus(store) => store.push(layer, key, value),
        }
    }

    pub(in crate::scalar) fn key(
        &mut self,
        layer: usize,
        position: usize,
    ) -> Result<&[f32], InferenceError> {
        match self {
            KvCacheStore::Vec(store) => store.key(layer, position),
            #[cfg(feature = "locus-kv")]
            KvCacheStore::Locus(store) => store.key(layer, position),
        }
    }

    pub(in crate::scalar) fn value(
        &mut self,
        layer: usize,
        position: usize,
    ) -> Result<&[f32], InferenceError> {
        match self {
            KvCacheStore::Vec(store) => store.value(layer, position),
            #[cfg(feature = "locus-kv")]
            KvCacheStore::Locus(store) => store.value(layer, position),
        }
    }

    pub(in crate::scalar) fn truncate(&mut self, token_count: usize) -> Result<(), InferenceError> {
        match self {
            KvCacheStore::Vec(store) => store.truncate(token_count),
            #[cfg(feature = "locus-kv")]
            KvCacheStore::Locus(store) => store.truncate(token_count),
        }
    }

    pub(in crate::scalar) fn kv_cache_bytes(&self) -> u128 {
        match self {
            KvCacheStore::Vec(store) => store.kv_cache_bytes(),
            #[cfg(feature = "locus-kv")]
            KvCacheStore::Locus(store) => store.kv_cache_bytes(),
        }
    }

    pub(in crate::scalar) fn snapshot(
        &mut self,
        cached_token_count: usize,
    ) -> Result<ScalarLlamaSessionSnapshot, InferenceError> {
        match self {
            KvCacheStore::Vec(store) => store.snapshot(cached_token_count),
            #[cfg(feature = "locus-kv")]
            KvCacheStore::Locus(store) => store.snapshot(cached_token_count),
        }
    }

    pub(in crate::scalar) fn restore(
        &mut self,
        snapshot: &ScalarLlamaSessionSnapshot,
    ) -> Result<(), InferenceError> {
        match self {
            KvCacheStore::Vec(store) => store.restore(snapshot),
            #[cfg(feature = "locus-kv")]
            KvCacheStore::Locus(store) => store.restore(snapshot),
        }
    }
}

/// Nested-`Vec` KV storage: one inner `Vec<f32>` per (layer, position).
#[derive(Debug)]
pub(in crate::scalar) struct VecKvStore {
    head_kv_dim: usize,
    layer_keys: Vec<Vec<Vec<f32>>>,
    layer_values: Vec<Vec<Vec<f32>>>,
}

impl VecKvStore {
    fn new(layer_count: usize, head_kv_dim: usize) -> Self {
        Self {
            head_kv_dim,
            layer_keys: vec![Vec::new(); layer_count],
            layer_values: vec![Vec::new(); layer_count],
        }
    }

    fn layer_count(&self) -> usize {
        self.layer_keys.len()
    }

    fn layer_len(&self, layer: usize) -> usize {
        self.layer_keys.get(layer).map_or(0, Vec::len)
    }

    fn check_dim(&self, label: &str, values: &[f32]) -> Result<(), InferenceError> {
        if values.len() != self.head_kv_dim {
            return Err(InferenceError::new(format!(
                "{label} length {} does not match head_kv_dim {}",
                values.len(),
                self.head_kv_dim
            )));
        }
        Ok(())
    }

    fn push(&mut self, layer: usize, key: &[f32], value: &[f32]) -> Result<(), InferenceError> {
        self.check_dim("key", key)?;
        self.check_dim("value", value)?;
        let keys = self
            .layer_keys
            .get_mut(layer)
            .ok_or_else(|| InferenceError::new(format!("kv layer {layer} out of bounds")))?;
        keys.push(key.to_vec());
        let values = self
            .layer_values
            .get_mut(layer)
            .ok_or_else(|| InferenceError::new(format!("kv layer {layer} out of bounds")))?;
        values.push(value.to_vec());
        Ok(())
    }

    fn key(&mut self, layer: usize, position: usize) -> Result<&[f32], InferenceError> {
        self.layer_keys
            .get(layer)
            .and_then(|layer| layer.get(position))
            .map(Vec::as_slice)
            .ok_or_else(|| InferenceError::new(format!("kv key ({layer},{position}) out of bounds")))
    }

    fn value(&mut self, layer: usize, position: usize) -> Result<&[f32], InferenceError> {
        self.layer_values
            .get(layer)
            .and_then(|layer| layer.get(position))
            .map(Vec::as_slice)
            .ok_or_else(|| {
                InferenceError::new(format!("kv value ({layer},{position}) out of bounds"))
            })
    }

    fn truncate(&mut self, token_count: usize) -> Result<(), InferenceError> {
        for keys in &mut self.layer_keys {
            keys.truncate(token_count);
        }
        for values in &mut self.layer_values {
            values.truncate(token_count);
        }
        Ok(())
    }

    fn kv_cache_bytes(&self) -> u128 {
        super::memory::kv_cache_bytes(&self.layer_keys, &self.layer_values)
    }

    fn snapshot(
        &mut self,
        cached_token_count: usize,
    ) -> Result<ScalarLlamaSessionSnapshot, InferenceError> {
        ScalarLlamaSessionSnapshot::from_layers(
            self.layer_keys.clone(),
            self.layer_values.clone(),
            cached_token_count,
        )
    }

    fn restore(&mut self, snapshot: &ScalarLlamaSessionSnapshot) -> Result<(), InferenceError> {
        if snapshot.layers_len() != self.layer_keys.len() {
            return Err(InferenceError::new(format!(
                "cache snapshot layer count does not match model layer count {}",
                self.layer_keys.len()
            )));
        }
        self.layer_keys = snapshot.layer_keys_owned();
        self.layer_values = snapshot.layer_values_owned();
        Ok(())
    }
}
```

- [ ] **Step 3b: Add snapshot constructor/accessors used by the store**

In `crates/ferrite-inference/src/scalar/session/snapshot.rs`, add these `pub(in crate::scalar)` items inside `impl ScalarLlamaSessionSnapshot` (they marshal to/from the owned form without changing the public struct):

```rust
    pub(in crate::scalar) fn from_layers(
        layer_keys: Vec<Vec<Vec<f32>>>,
        layer_values: Vec<Vec<Vec<f32>>>,
        cached_token_count: usize,
    ) -> Result<Self, InferenceError> {
        if layer_keys.len() != layer_values.len() {
            return Err(InferenceError::new(
                "snapshot key and value layer counts differ",
            ));
        }
        Ok(Self {
            layer_keys,
            layer_values,
            cached_token_count,
        })
    }

    pub(in crate::scalar) fn layers_len(&self) -> usize {
        self.layer_keys.len()
    }

    pub(in crate::scalar) fn layer_keys_owned(&self) -> Vec<Vec<Vec<f32>>> {
        self.layer_keys.clone()
    }

    pub(in crate::scalar) fn layer_values_owned(&self) -> Vec<Vec<Vec<f32>>> {
        self.layer_values.clone()
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p ferrite-inference --lib scalar::kv_store`
Expected: PASS (3 tests).

Run: `cargo clippy -p ferrite-inference --all-targets`
Expected: no `unwrap_used`/`expect_used`/`panic` violations.

- [ ] **Step 5: Commit**

```bash
git add crates/ferrite-inference/src/scalar.rs crates/ferrite-inference/src/scalar/kv_store.rs crates/ferrite-inference/src/scalar/session/snapshot.rs
git commit -m "feat: add KvCacheStore seam with Vec backend"
```

---

### Task 3: Route `ScalarLlamaSession` and attention through the store (Vec only, no behavior change)

**Files:**
- Modify: `crates/ferrite-inference/src/scalar/session.rs:15-22` (struct), `:316-324` (push + attention call)
- Modify: `crates/ferrite-inference/src/scalar/attention.rs:6-59` (read via store)
- Modify: `crates/ferrite-inference/src/scalar/session/cache.rs` (`new`, `truncate_cache`, `kv_cache_bytes`)
- Modify: `crates/ferrite-inference/src/scalar/session/snapshot.rs` (`cache_snapshot`, `restore_cache_snapshot`)

**Interfaces:**
- Consumes: `KvCacheStore` (Task 2).
- Produces: `causal_attention(config, query, store: &mut KvCacheStore, layer: usize) -> Result<Vec<f32>, InferenceError>`.

- [ ] **Step 1: Refactor `causal_attention` to read from the store**

Replace the body of `causal_attention` in `crates/ferrite-inference/src/scalar/attention.rs` (keep the imports; add the store import). New signature and body — numeric operations are identical, only the source of key/value slices changes:

```rust
use super::{
    kv_store::KvCacheStore,
    math::{dot, ensure_len, softmax},
    InferenceError, ScalarLlamaConfig,
};

pub(super) fn causal_attention(
    config: &ScalarLlamaConfig,
    query: &[f32],
    store: &mut KvCacheStore,
    layer: usize,
) -> Result<Vec<f32>, InferenceError> {
    let expected_query = config.attention_head_count * config.head_dim;
    let expected_kv = config.attention_head_count_kv * config.head_dim;
    ensure_len("query", query, expected_query)?;

    let position_count = store.layer_len(layer);
    if position_count == 0 {
        return Err(InferenceError::new("attention cache must not be empty"));
    }

    let heads_per_kv = config
        .attention_head_count
        .checked_div(config.attention_head_count_kv)
        .ok_or_else(|| InferenceError::new("invalid zero kv head count"))?;

    let mut output = vec![0.0; expected_query];
    for query_head in 0..config.attention_head_count {
        let kv_head = query_head / heads_per_kv;
        let query_start = query_head * config.head_dim;
        let kv_start = kv_head * config.head_dim;
        let query_slice = &query[query_start..query_start + config.head_dim];

        let mut scores = Vec::with_capacity(position_count);
        for position in 0..position_count {
            let key = store.key(layer, position)?;
            ensure_len("cached key", key, expected_kv)?;
            let key_slice = &key[kv_start..kv_start + config.head_dim];
            scores.push(dot(query_slice, key_slice)? / (config.head_dim as f32).sqrt());
        }

        let weights = softmax(&scores)?;
        for position in 0..position_count {
            let value = store.value(layer, position)?;
            ensure_len("cached value", value, expected_kv)?;
            let value_slice = &value[kv_start..kv_start + config.head_dim];
            if value_slice.iter().any(|value| !value.is_finite()) {
                return Err(InferenceError::new("cached value must be finite"));
            }
            for dimension in 0..config.head_dim {
                output[query_start + dimension] += weights[position] * value_slice[dimension];
            }
        }
    }

    Ok(output)
}
```

- [ ] **Step 2: Update the attention unit tests to build a store**

Replace the two tests in `attention.rs`'s `tests` module so they construct a single-layer `KvCacheStore::new_vec` instead of `Vec<Vec<f32>>`. New `tests` module body:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::scalar::kv_store::KvCacheStore;
    use crate::scalar::RopeLayout;

    fn config_for_ratio(heads_per_kv: usize) -> ScalarLlamaConfig {
        let kv_heads = 2;
        let head_dim = 2;
        ScalarLlamaConfig {
            vocab_size: 1,
            hidden_size: heads_per_kv * kv_heads * head_dim,
            intermediate_size: 1,
            attention_head_count: heads_per_kv * kv_heads,
            attention_head_count_kv: kv_heads,
            head_dim,
            rope_dimension_count: 0,
            rope_freq_base: 10_000.0,
            rope_layout: RopeLayout::AdjacentPairs,
            rms_norm_epsilon: 0.0,
        }
    }

    fn single_position_store(config: &ScalarLlamaConfig, value: Vec<f32>) -> Result<KvCacheStore, InferenceError> {
        let dim = config.attention_head_count_kv * config.head_dim;
        let mut store = KvCacheStore::new_vec(1, dim);
        store.push(0, &vec![0.0; dim], &value)?;
        Ok(store)
    }

    #[test]
    fn gqa_broadcasts_kv_heads_for_tier1_ratios() -> Result<(), InferenceError> {
        for heads_per_kv in [1, 3, 4, 6, 7] {
            let config = config_for_ratio(heads_per_kv);
            let query = vec![1.0; config.hidden_size];
            let value = vec![10.0, 11.0, 20.0, 21.0];
            let mut store = single_position_store(&config, value.clone())?;

            let output = causal_attention(&config, &query, &mut store, 0)?;

            for query_head in 0..config.attention_head_count {
                let kv_head = query_head / heads_per_kv;
                let output_start = query_head * config.head_dim;
                let kv_start = kv_head * config.head_dim;
                assert_eq!(
                    &output[output_start..output_start + config.head_dim],
                    &value[kv_start..kv_start + config.head_dim],
                    "heads_per_kv={heads_per_kv}, query_head={query_head}"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn attention_rejects_non_finite_cached_values() -> Result<(), InferenceError> {
        let config = config_for_ratio(1);
        let query = vec![1.0; config.hidden_size];

        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let dim = config.attention_head_count_kv * config.head_dim;
            let mut values = vec![0.0; dim];
            values[0] = value;
            let mut store = single_position_store(&config, values)?;

            let error = match causal_attention(&config, &query, &mut store, 0) {
                Ok(_) => return Err(InferenceError::new("non-finite cached value should fail")),
                Err(error) => error,
            };

            assert!(error.to_string().contains("cached value must be finite"));
        }
        Ok(())
    }
}
```

Note: `KvCacheStore` must be reachable as `crate::scalar::kv_store::KvCacheStore` from tests. Ensure `mod kv_store;` in `scalar.rs` is not `pub` — `pub(in crate::scalar)` items are visible to sibling modules and their `#[cfg(test)]` submodules, which is sufficient.

- [ ] **Step 3: Swap the session struct fields to the store**

In `crates/ferrite-inference/src/scalar/session.rs`, change the struct (lines 15-22):

```rust
#[derive(Debug)]
pub struct ScalarLlamaSession<'a> {
    model: &'a ScalarLlamaModel,
    store: super::kv_store::KvCacheStore,
    cached_token_count: usize,
    options: ScalarExecutionOptions,
}
```

- [ ] **Step 4: Update push + attention call in `accept_token_inner`**

Replace lines 316-324 of `session.rs`:

```rust
            self.store.push(layer_index, &key, &value)?;

            let attention = causal_attention(
                &self.model.config,
                &query,
                &mut self.store,
                layer_index,
            )?;
```

(`self.model.config` and `self.store` are disjoint fields — the immutable borrow of `self.model` and the mutable borrow of `self.store` coexist.)

- [ ] **Step 5: Update `cache.rs` constructors and helpers**

Replace `crates/ferrite-inference/src/scalar/session/cache.rs` body of the `impl` with store-based construction. `new` stays infallible (Vec default):

```rust
use super::ScalarLlamaSession;
use crate::scalar::kv_store::KvCacheStore;
use crate::scalar::{InferenceError, ScalarExecutionOptions, ScalarLlamaModel};

impl<'a> ScalarLlamaSession<'a> {
    pub(in crate::scalar) fn new(model: &'a ScalarLlamaModel) -> Self {
        let head_kv_dim = model.config.attention_head_count_kv * model.config.head_dim;
        Self {
            model,
            store: KvCacheStore::new_vec(model.weights.layers.len(), head_kv_dim),
            cached_token_count: 0,
            options: ScalarExecutionOptions::default(),
        }
    }

    pub(in crate::scalar) fn from_store(
        model: &'a ScalarLlamaModel,
        store: KvCacheStore,
        options: ScalarExecutionOptions,
    ) -> Self {
        Self {
            model,
            store,
            cached_token_count: 0,
            options,
        }
    }

    pub fn cached_token_count(&self) -> usize {
        self.cached_token_count
    }

    pub fn kv_cache_bytes(&self) -> u128 {
        self.store.kv_cache_bytes()
    }

    pub fn truncate_cache(&mut self, token_count: usize) -> Result<(), InferenceError> {
        if token_count > self.cached_token_count {
            return Err(InferenceError::new(format!(
                "cannot truncate kv cache from {} tokens to {token_count} tokens",
                self.cached_token_count
            )));
        }
        self.store.truncate(token_count)?;
        self.cached_token_count = token_count;
        Ok(())
    }
}
```

(The old `new_with_options` moves to Task 6, which owns backend selection and fallibility.)

- [ ] **Step 6: Update `snapshot.rs` session methods**

Replace the `impl ScalarLlamaSession` block (lines 48-91) of `snapshot.rs` to go through the store:

```rust
impl<'a> ScalarLlamaSession<'a> {
    pub fn cache_snapshot(&mut self) -> Result<ScalarLlamaSessionSnapshot, InferenceError> {
        self.store.snapshot(self.cached_token_count)
    }

    pub fn restore_cache_snapshot(
        &mut self,
        snapshot: &ScalarLlamaSessionSnapshot,
    ) -> Result<(), InferenceError> {
        let expected_layers = self.model.weights.layers.len();
        if snapshot.layers_len() != expected_layers {
            return Err(InferenceError::new(format!(
                "cache snapshot layer count does not match model layer count {expected_layers}"
            )));
        }
        self.store.restore(snapshot)?;
        self.cached_token_count = snapshot.cached_token_count();
        Ok(())
    }
}
```

Note: `cache_snapshot` now takes `&mut self` (the Locus backend needs it to read blocks) and returns `Result`. Update callers in the next step.

- [ ] **Step 7: Update `cache_snapshot` callers**

Find callers: `grep -rn "cache_snapshot" crates --include="*.rs"`. Update each to `&mut` + `?`. Known site: `crates/ferrite-server/src/runtime.rs` (stores `session.cache_snapshot()`), plus tests in `crates/ferrite-inference/tests/scalar_session_cache.rs`. For each call `session.cache_snapshot()` → `session.cache_snapshot()?` and ensure the session binding is `mut`. In `runtime.rs`, the session is already `let mut session`, so only add `?` and propagate (the enclosing function already returns a `Result`).

- [ ] **Step 8: Run the full existing test suite (proves no behavior change)**

Run: `cargo test -p ferrite-inference`
Expected: PASS — `scalar_reference`, `scalar_session_cache`, `scalar_profile`, `scalar_prompt_cancellation`, and the in-crate tests all pass unchanged.

Run: `cargo test -p ferrite-server`
Expected: PASS.

Run: `cargo clippy --workspace --all-targets`
Expected: clean.

- [ ] **Step 9: Commit**

```bash
git add crates/ferrite-inference/src/scalar/session.rs crates/ferrite-inference/src/scalar/attention.rs crates/ferrite-inference/src/scalar/session/cache.rs crates/ferrite-inference/src/scalar/session/snapshot.rs crates/ferrite-server/src/runtime.rs
git commit -m "refactor: route session KV through KvCacheStore (Vec backend, no behavior change)"
```

---

### Task 4: Add optional Locus + bytemuck dependencies and the `locus-kv` feature

**Files:**
- Modify: `crates/ferrite-inference/Cargo.toml`

- [ ] **Step 1: Add optional deps and feature**

Edit `crates/ferrite-inference/Cargo.toml`:

```toml
[dependencies]
ferrite-model = { path = "../ferrite-model" }
rayon = "1.10"
locus-alloc = { path = "../../../locus/crates/locus-alloc", optional = true }
bytemuck = { version = "1", optional = true }

[features]
locus-kv = ["dep:locus-alloc", "dep:bytemuck"]
```

Note on the path: adjust `../../../locus/crates/locus-alloc` if the `locus` checkout is not a sibling of the `ferrite` checkout. Verify with: `ls ../../../locus/crates/locus-alloc/Cargo.toml` from `crates/ferrite-inference/`. If Locus is published, `locus-alloc = { version = "0.1", optional = true }` is an alternative.

- [ ] **Step 2: Verify both feature states build**

Run: `cargo build -p ferrite-inference`
Expected: builds (feature off; no new deps compiled).

Run: `cargo build -p ferrite-inference --features locus-kv`
Expected: FAIL — `super::kv_store::locus::LocusKvStore` referenced by the `#[cfg(feature = "locus-kv")]` enum arm does not exist yet. This confirms the feature wiring reaches the (not-yet-written) module. (Task 5 creates it.)

- [ ] **Step 3: Commit**

```bash
git add crates/ferrite-inference/Cargo.toml Cargo.lock
git commit -m "build: add optional locus-alloc + bytemuck deps behind locus-kv feature"
```

---

### Task 5: Implement `LocusKvStore`

**Files:**
- Create: `crates/ferrite-inference/src/scalar/kv_store/locus.rs`
- Modify: `crates/ferrite-inference/src/scalar/kv_store.rs` (declare the submodule under the feature)

**Interfaces:**
- Consumes: `locus_alloc::{KvBlockPool, KvBlockHandle, KvReuseOrder, NodeId, KvBlockPoolError}`, `bytemuck`, `ScalarLlamaSessionSnapshot`, `InferenceError`.
- Produces: `LocusKvStore` with the same private method surface as `VecKvStore` (`layer_count`, `layer_len`, `push`, `key`, `value`, `truncate`, `kv_cache_bytes`, `snapshot`, `restore`) plus `LocusKvStore::new(layer_count, head_kv_dim, tokens_per_block, max_tokens) -> Result<Self, InferenceError>` and `pool_stats()` for benchmarking.

- [ ] **Step 1: Declare the submodule**

In `crates/ferrite-inference/src/scalar/kv_store.rs`, add near the top:

```rust
#[cfg(feature = "locus-kv")]
pub(in crate::scalar) mod locus;
```

- [ ] **Step 2: Write failing tests**

Create `crates/ferrite-inference/src/scalar/kv_store/locus.rs` with the tests first:

```rust
#[cfg(test)]
mod tests {
    use super::LocusKvStore;
    use crate::scalar::InferenceError;

    fn sample(layer: usize, position: usize, dim: usize) -> Vec<f32> {
        (0..dim)
            .map(|d| (layer * 1000 + position * 10 + d) as f32 + 0.5)
            .collect()
    }

    #[test]
    fn locus_store_round_trips_across_block_boundaries() -> Result<(), InferenceError> {
        let dim = 4;
        // tokens_per_block = 2 forces multiple blocks for 5 positions.
        let mut store = LocusKvStore::new(2, dim, 2, 8)?;
        for position in 0..5 {
            for layer in 0..2 {
                store.push(layer, &sample(layer, position, dim), &sample(layer + 100, position, dim))?;
            }
        }
        for layer in 0..2 {
            assert_eq!(store.layer_len(layer), 5);
            for position in 0..5 {
                assert_eq!(store.key(layer, position)?, sample(layer, position, dim).as_slice());
                assert_eq!(store.value(layer, position)?, sample(layer + 100, position, dim).as_slice());
            }
        }
        Ok(())
    }

    #[test]
    fn locus_store_truncates_and_frees_blocks() -> Result<(), InferenceError> {
        let dim = 2;
        let mut store = LocusKvStore::new(1, dim, 2, 8)?;
        for position in 0..4 {
            store.push(0, &sample(0, position, dim), &sample(0, position, dim))?;
        }
        let allocated_before = store.pool_stats().allocated;
        store.truncate(1)?;
        assert_eq!(store.layer_len(0), 1);
        assert!(store.pool_stats().allocated < allocated_before);
        assert!(store.key(0, 1).is_err());
        Ok(())
    }

    #[test]
    fn locus_store_reports_out_of_blocks() -> Result<(), InferenceError> {
        let dim = 2;
        // capacity sized for 2 tokens; pushing a 3rd must error.
        let mut store = LocusKvStore::new(1, dim, 1, 2)?;
        store.push(0, &sample(0, 0, dim), &sample(0, 0, dim))?;
        store.push(0, &sample(0, 1, dim), &sample(0, 1, dim))?;
        let error = match store.push(0, &sample(0, 2, dim), &sample(0, 2, dim)) {
            Ok(()) => return Err(InferenceError::new("expected out-of-blocks error")),
            Err(error) => error,
        };
        assert!(error.to_string().contains("out of blocks") || error.to_string().contains("OutOfBlocks"));
        Ok(())
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p ferrite-inference --features locus-kv --lib scalar::kv_store::locus`
Expected: FAIL — `LocusKvStore` not found.

- [ ] **Step 4: Implement `LocusKvStore`**

Above the `tests` module in `locus.rs`:

```rust
use locus_alloc::{KvBlockHandle, KvBlockPool, KvBlockPoolError, KvReuseOrder, NodeId};

use crate::scalar::session::ScalarLlamaSessionSnapshot;
use crate::scalar::InferenceError;

const F32_BYTES: usize = std::mem::size_of::<f32>();

/// Locus-backed KV storage: fixed-size, mapped, page-aligned blocks holding
/// `tokens_per_block` positions each, one block list per (layer, K|V).
#[derive(Debug)]
pub(in crate::scalar) struct LocusKvStore {
    pool: KvBlockPool,
    head_kv_dim: usize,
    tokens_per_block: usize,
    key_blocks: Vec<Vec<KvBlockHandle>>,
    value_blocks: Vec<Vec<KvBlockHandle>>,
    layer_len: Vec<usize>,
}

fn map_pool_error(error: KvBlockPoolError) -> InferenceError {
    InferenceError::new(format!("locus kv pool error: {error}"))
}

impl LocusKvStore {
    pub(in crate::scalar) fn new(
        layer_count: usize,
        head_kv_dim: usize,
        tokens_per_block: usize,
        max_tokens: usize,
    ) -> Result<Self, InferenceError> {
        if head_kv_dim == 0 {
            return Err(InferenceError::new("locus kv head_kv_dim must be non-zero"));
        }
        if tokens_per_block == 0 {
            return Err(InferenceError::new("locus kv tokens_per_block must be non-zero"));
        }
        if max_tokens == 0 {
            return Err(InferenceError::new("locus kv max_tokens must be non-zero"));
        }
        let block_size = tokens_per_block
            .checked_mul(head_kv_dim)
            .and_then(|n| n.checked_mul(F32_BYTES))
            .ok_or_else(|| InferenceError::new("locus kv block size overflow"))?;
        let blocks_per_layer = max_tokens.div_ceil(tokens_per_block);
        // Two block lists (K and V) per layer.
        let capacity = blocks_per_layer
            .checked_mul(layer_count)
            .and_then(|n| n.checked_mul(2))
            .ok_or_else(|| InferenceError::new("locus kv capacity overflow"))?
            .max(1);
        let pool = KvBlockPool::new_mapped(NodeId(0), block_size, capacity, KvReuseOrder::Lifo)
            .map_err(map_pool_error)?;
        Ok(Self {
            pool,
            head_kv_dim,
            tokens_per_block,
            key_blocks: vec![Vec::new(); layer_count],
            value_blocks: vec![Vec::new(); layer_count],
            layer_len: vec![0; layer_count],
        })
    }

    pub(in crate::scalar) fn pool_stats(&self) -> locus_alloc::KvBlockPoolStats {
        self.pool.stats()
    }

    fn layer_count(&self) -> usize {
        self.layer_len.len()
    }

    fn layer_len(&self, layer: usize) -> usize {
        self.layer_len.get(layer).copied().unwrap_or(0)
    }

    fn byte_range(&self, position: usize) -> (usize, usize) {
        let within = position % self.tokens_per_block;
        let start = within * self.head_kv_dim * F32_BYTES;
        (start, start + self.head_kv_dim * F32_BYTES)
    }

    fn write_block(
        &mut self,
        blocks_are_keys: bool,
        layer: usize,
        position: usize,
        values: &[f32],
    ) -> Result<(), InferenceError> {
        let (start, end) = self.byte_range(position);
        let block_index = position / self.tokens_per_block;
        let handle = {
            let blocks = if blocks_are_keys {
                &self.key_blocks
            } else {
                &self.value_blocks
            };
            let layer_blocks = blocks
                .get(layer)
                .ok_or_else(|| InferenceError::new(format!("locus kv layer {layer} out of bounds")))?;
            *layer_blocks
                .get(block_index)
                .ok_or_else(|| InferenceError::new("locus kv block index out of bounds"))?
        };
        let bytes = self.pool.block_mut(handle).map_err(map_pool_error)?;
        let slot: &mut [f32] = bytemuck::try_cast_slice_mut(&mut bytes[start..end])
            .map_err(|error| InferenceError::new(format!("locus kv cast error: {error}")))?;
        slot.copy_from_slice(values);
        Ok(())
    }

    fn ensure_block(&mut self, layer: usize, position: usize) -> Result<(), InferenceError> {
        if position % self.tokens_per_block != 0 {
            return Ok(());
        }
        let key_handle = self.pool.allocate().map_err(map_pool_error)?;
        let value_handle = self.pool.allocate().map_err(map_pool_error)?;
        self.key_blocks
            .get_mut(layer)
            .ok_or_else(|| InferenceError::new(format!("locus kv layer {layer} out of bounds")))?
            .push(key_handle);
        self.value_blocks
            .get_mut(layer)
            .ok_or_else(|| InferenceError::new(format!("locus kv layer {layer} out of bounds")))?
            .push(value_handle);
        Ok(())
    }

    fn push(&mut self, layer: usize, key: &[f32], value: &[f32]) -> Result<(), InferenceError> {
        if key.len() != self.head_kv_dim || value.len() != self.head_kv_dim {
            return Err(InferenceError::new(format!(
                "locus kv push expects head_kv_dim {}, got key {} value {}",
                self.head_kv_dim,
                key.len(),
                value.len()
            )));
        }
        let position = self.layer_len(layer);
        self.ensure_block(layer, position)?;
        self.write_block(true, layer, position, key)?;
        self.write_block(false, layer, position, value)?;
        if let Some(len) = self.layer_len.get_mut(layer) {
            *len += 1;
        }
        Ok(())
    }

    fn read_block(
        &mut self,
        blocks_are_keys: bool,
        layer: usize,
        position: usize,
    ) -> Result<&[f32], InferenceError> {
        if position >= self.layer_len(layer) {
            return Err(InferenceError::new(format!(
                "locus kv position {position} out of bounds for layer {layer}"
            )));
        }
        let (start, end) = self.byte_range(position);
        let block_index = position / self.tokens_per_block;
        let handle = {
            let blocks = if blocks_are_keys {
                &self.key_blocks
            } else {
                &self.value_blocks
            };
            *blocks
                .get(layer)
                .and_then(|layer_blocks| layer_blocks.get(block_index))
                .ok_or_else(|| InferenceError::new("locus kv block index out of bounds"))?
        };
        let bytes = self.pool.block_mut(handle).map_err(map_pool_error)?;
        bytemuck::try_cast_slice(&bytes[start..end])
            .map_err(|error| InferenceError::new(format!("locus kv cast error: {error}")))
    }

    fn key(&mut self, layer: usize, position: usize) -> Result<&[f32], InferenceError> {
        self.read_block(true, layer, position)
    }

    fn value(&mut self, layer: usize, position: usize) -> Result<&[f32], InferenceError> {
        self.read_block(false, layer, position)
    }

    fn truncate(&mut self, token_count: usize) -> Result<(), InferenceError> {
        let needed_blocks = token_count.div_ceil(self.tokens_per_block);
        for layer in 0..self.layer_count() {
            for blocks in [&mut self.key_blocks, &mut self.value_blocks] {
                if let Some(layer_blocks) = blocks.get_mut(layer) {
                    while layer_blocks.len() > needed_blocks {
                        if let Some(handle) = layer_blocks.pop() {
                            self.pool.free(handle).map_err(map_pool_error)?;
                        }
                    }
                }
            }
            if let Some(len) = self.layer_len.get_mut(layer) {
                *len = (*len).min(token_count);
            }
        }
        Ok(())
    }

    fn kv_cache_bytes(&self) -> u128 {
        // Logical f32 bytes, identical semantics to the Vec backend.
        let per_position = (self.head_kv_dim * F32_BYTES) as u128;
        self.layer_len
            .iter()
            .map(|len| *len as u128 * per_position * 2)
            .sum()
    }

    fn snapshot(
        &mut self,
        cached_token_count: usize,
    ) -> Result<ScalarLlamaSessionSnapshot, InferenceError> {
        let layer_count = self.layer_count();
        let mut layer_keys = Vec::with_capacity(layer_count);
        let mut layer_values = Vec::with_capacity(layer_count);
        for layer in 0..layer_count {
            let len = self.layer_len(layer);
            let mut keys = Vec::with_capacity(len);
            let mut values = Vec::with_capacity(len);
            for position in 0..len {
                keys.push(self.key(layer, position)?.to_vec());
                values.push(self.value(layer, position)?.to_vec());
            }
            layer_keys.push(keys);
            layer_values.push(values);
        }
        ScalarLlamaSessionSnapshot::from_layers(layer_keys, layer_values, cached_token_count)
    }

    fn restore(&mut self, snapshot: &ScalarLlamaSessionSnapshot) -> Result<(), InferenceError> {
        if snapshot.layers_len() != self.layer_count() {
            return Err(InferenceError::new(format!(
                "cache snapshot layer count does not match model layer count {}",
                self.layer_count()
            )));
        }
        // Free everything, then re-push all positions (cold path).
        self.truncate(0)?;
        let keys = snapshot.layer_keys_owned();
        let values = snapshot.layer_values_owned();
        for (layer, (layer_keys, layer_values)) in keys.iter().zip(values.iter()).enumerate() {
            for (key, value) in layer_keys.iter().zip(layer_values.iter()) {
                self.push(layer, key, value)?;
            }
        }
        Ok(())
    }
}
```

Re-export `KvBlockPoolStats` visibility: it is already public in `locus_alloc`; `pool_stats` returns it directly. Add `pub use` if a later task needs it outside the module (Task 8 accesses it through a session method — see Step 6).

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p ferrite-inference --features locus-kv --lib scalar::kv_store::locus`
Expected: PASS (3 tests).

Run: `cargo clippy -p ferrite-inference --features locus-kv --all-targets`
Expected: clean (no unwrap/expect/panic/unsafe).

- [ ] **Step 6: Expose a benchmark hook on the session (feature-gated)**

In `crates/ferrite-inference/src/scalar/session/cache.rs`, add a feature-gated accessor so the CLI can read pool stats:

```rust
    #[cfg(feature = "locus-kv")]
    pub fn locus_pool_allocation_count(&self) -> Option<u64> {
        match &self.store {
            crate::scalar::kv_store::KvCacheStore::Locus(store) => {
                Some(store.pool_stats().allocation_count)
            }
            crate::scalar::kv_store::KvCacheStore::Vec(_) => None,
        }
    }
```

Run: `cargo build -p ferrite-inference --features locus-kv`
Expected: builds.

- [ ] **Step 7: Commit**

```bash
git add crates/ferrite-inference/src/scalar/kv_store.rs crates/ferrite-inference/src/scalar/kv_store/locus.rs crates/ferrite-inference/src/scalar/session/cache.rs
git commit -m "feat: implement LocusKvStore block-pool KV backend"
```

---

### Task 6: Backend selection in a fallible `new_with_options` + differential/parity tests

**Files:**
- Modify: `crates/ferrite-inference/src/scalar/session/cache.rs` (fallible `new_with_options`)
- Modify: `crates/ferrite-inference/src/scalar.rs` (`start_session_with_options` → `Result`)
- Modify: `crates/ferrite-cli/src/run.rs:59`, `crates/ferrite-cli/src/benchmark.rs:16`, `crates/ferrite-inference/tests/scalar_reference.rs:427,457` (handle `Result`)
- Create: `crates/ferrite-inference/tests/kv_store_backend_parity.rs`

**Interfaces:**
- Produces: `ScalarLlamaSession::new_with_options(model, options) -> Result<Self, InferenceError>`; `ScalarLlamaModel::start_session_with_options(&self, options) -> Result<ScalarLlamaSession<'_>, InferenceError>`.

- [ ] **Step 1: Write the failing backend-selection test**

Add to `crates/ferrite-inference/src/scalar/kv_store.rs` `tests` module:

```rust
    #[cfg(feature = "locus-kv")]
    #[test]
    fn build_from_backend_selects_locus() -> Result<(), crate::scalar::InferenceError> {
        use crate::scalar::options::KvBackend;
        let store = KvCacheStore::from_backend(
            2,
            4,
            KvBackend::Locus { tokens_per_block: 16, max_tokens: 64 },
        )?;
        assert!(matches!(store, KvCacheStore::Locus(_)));
        Ok(())
    }

    #[test]
    fn build_from_backend_defaults_to_vec() -> Result<(), crate::scalar::InferenceError> {
        use crate::scalar::options::KvBackend;
        let store = KvCacheStore::from_backend(2, 4, KvBackend::Vec)?;
        assert!(matches!(store, KvCacheStore::Vec(_)));
        Ok(())
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p ferrite-inference --lib scalar::kv_store::tests::build_from_backend_defaults_to_vec`
Expected: FAIL — `from_backend` not found.

- [ ] **Step 3: Add `KvCacheStore::from_backend`**

In `kv_store.rs`, add to `impl KvCacheStore`:

```rust
    pub(in crate::scalar) fn from_backend(
        layer_count: usize,
        head_kv_dim: usize,
        backend: super::options::KvBackend,
    ) -> Result<Self, InferenceError> {
        match backend {
            super::options::KvBackend::Vec => Ok(Self::new_vec(layer_count, head_kv_dim)),
            #[cfg(feature = "locus-kv")]
            super::options::KvBackend::Locus { tokens_per_block, max_tokens } => Ok(
                KvCacheStore::Locus(locus::LocusKvStore::new(
                    layer_count,
                    head_kv_dim,
                    tokens_per_block,
                    max_tokens,
                )?),
            ),
            #[cfg(not(feature = "locus-kv"))]
            super::options::KvBackend::Locus { .. } => Err(InferenceError::new(
                "locus kv backend requested but the `locus-kv` feature is not enabled",
            )),
        }
    }
```

(Import path note: `KvBackend` lives in `super::options`; the enum is `pub`. Ensure `options` is reachable as `super::options` from `kv_store` — both are children of `scalar`.)

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p ferrite-inference --lib scalar::kv_store`
Expected: PASS. Also `cargo test -p ferrite-inference --features locus-kv --lib scalar::kv_store` PASS.

- [ ] **Step 5: Add the fallible `new_with_options`**

In `crates/ferrite-inference/src/scalar/session/cache.rs`, add to the `impl`:

```rust
    pub(in crate::scalar) fn new_with_options(
        model: &'a ScalarLlamaModel,
        options: ScalarExecutionOptions,
    ) -> Result<Self, InferenceError> {
        let head_kv_dim = model.config.attention_head_count_kv * model.config.head_dim;
        let store = KvCacheStore::from_backend(
            model.weights.layers.len(),
            head_kv_dim,
            options.kv_backend(),
        )?;
        Ok(Self::from_store(model, store, options))
    }
```

- [ ] **Step 6: Make `start_session_with_options` fallible**

In `crates/ferrite-inference/src/scalar.rs` (lines ~139-144):

```rust
    pub fn start_session_with_options(
        &self,
        options: ScalarExecutionOptions,
    ) -> Result<ScalarLlamaSession<'_>, InferenceError> {
        ScalarLlamaSession::new_with_options(self, options)
    }
```

Ensure `InferenceError` is in scope in `scalar.rs` (it is, via the module's existing re-exports/`use`).

- [ ] **Step 7: Update the callers**

`crates/ferrite-cli/src/run.rs:59`:

```rust
    let mut session = model.start_session_with_options(execution_options)?;
```

`crates/ferrite-cli/src/benchmark.rs:16`:

```rust
    let mut session = model.start_session_with_options(options)?;
```

(Both enclosing functions already return `Result`; if `benchmark.rs`'s does not, wrap its error — check the signature and add `?` propagation, converting via the same error type it already returns.)

`crates/ferrite-inference/tests/scalar_reference.rs:427` and `:457`: add `?` to the `start_session_with_options(...)` calls (those test functions return `Result`).

- [ ] **Step 8: Write the differential + fixture-parity integration test**

Create `crates/ferrite-inference/tests/kv_store_backend_parity.rs`. This uses the deterministic fixture model from `ferrite-fixtures` (the same fixtures the existing `scalar_reference.rs` uses — mirror its model-construction helper). Replace `build_fixture_model()` and `PROMPT` below with the exact fixture constructor used in `scalar_reference.rs` (open that file and copy the helper that yields a `ScalarLlamaModel`).

```rust
use ferrite_inference::scalar::{KvBackend, ScalarExecutionOptions};
use ferrite_inference::InferenceError;

mod support {
    // Copy the fixture model builder from tests/scalar_reference.rs here,
    // exposing `pub fn build_fixture_model() -> ferrite_inference::scalar::ScalarLlamaModel`
    // and `pub const PROMPT: &[usize]`.
}

#[cfg(feature = "locus-kv")]
#[test]
fn locus_backend_matches_vec_logits() -> Result<(), InferenceError> {
    let model = support::build_fixture_model();

    let mut vec_session = model.start_session_with_options(ScalarExecutionOptions::default())?;
    let vec_next = vec_session.accept_prompt(support::PROMPT)?;

    let locus_options = ScalarExecutionOptions::default().with_kv_backend(KvBackend::Locus {
        tokens_per_block: 4,
        max_tokens: 64,
    });
    let mut locus_session = model.start_session_with_options(locus_options)?;
    let locus_next = locus_session.accept_prompt(support::PROMPT)?;

    assert_eq!(vec_next.token_id, locus_next.token_id);
    assert_eq!(vec_next.logits, locus_next.logits);
    assert_eq!(vec_session.kv_cache_bytes(), locus_session.kv_cache_bytes());
    Ok(())
}
```

Note: `ScalarLlamaModel`, `KvBackend`, and `ScalarExecutionOptions` must be re-exported from `ferrite_inference::scalar` (verify `KvBackend` re-export from Task 1 Step 5; add `ScalarLlamaModel` if not already public).

- [ ] **Step 9: Run everything**

Run: `cargo test -p ferrite-inference`
Expected: PASS (Vec path unchanged; the `#[cfg(feature = "locus-kv")]` parity test is skipped).

Run: `cargo test -p ferrite-inference --features locus-kv`
Expected: PASS including `locus_backend_matches_vec_logits`.

Run: `cargo build -p ferrite-cli && cargo build -p ferrite-cli --features ferrite-inference/locus-kv` (see Task 7 for surfacing the feature through the CLI crate).
Expected: builds.

- [ ] **Step 10: Commit**

```bash
git add crates/ferrite-inference/src/scalar/session/cache.rs crates/ferrite-inference/src/scalar.rs crates/ferrite-inference/src/scalar/kv_store.rs crates/ferrite-cli/src/run.rs crates/ferrite-cli/src/benchmark.rs crates/ferrite-inference/tests/scalar_reference.rs crates/ferrite-inference/tests/kv_store_backend_parity.rs
git commit -m "feat: select KV backend via options with Vec/Locus differential parity tests"
```

---

### Task 7: CLI flags to select the Locus backend

**Files:**
- Modify: `crates/ferrite-cli/src/args.rs`, `crates/ferrite-cli/src/run.rs`, `crates/ferrite-cli/src/benchmark.rs`, `crates/ferrite-cli/Cargo.toml`

**Interfaces:**
- Consumes: `KvBackend`, `ScalarExecutionOptions::with_kv_backend`.

- [ ] **Step 1: Surface the feature through the CLI crate**

In `crates/ferrite-cli/Cargo.toml`, add a passthrough feature:

```toml
[features]
locus-kv = ["ferrite-inference/locus-kv"]
```

- [ ] **Step 2: Add args**

In `crates/ferrite-cli/src/args.rs`, add three optional fields to the parsed args struct (match the file's existing arg style — this repo hand-rolls arg parsing; mirror an existing optional flag like `--generate-tokens`):

- `--kv-backend <vec|locus>` (default `vec`)
- `--kv-tokens-per-block <usize>` (default `16`)
- `--kv-max-tokens <usize>` (default: reuse the effective generation cap, else `2048`)

Parse `--kv-backend` into a small enum `CliKvBackend { Vec, Locus }` with an error on unknown values, following the existing `parse`-style helpers (see `Q8KActivationMatvecRole::parse` for the pattern).

- [ ] **Step 3: Build `KvBackend` in run.rs/benchmark.rs**

Where `execution_options` / `options` are constructed (run.rs before line 59; benchmark.rs before line 16), add:

```rust
    let execution_options = match args.kv_backend {
        CliKvBackend::Vec => execution_options,
        CliKvBackend::Locus => execution_options.with_kv_backend(
            ferrite_inference::scalar::KvBackend::Locus {
                tokens_per_block: args.kv_tokens_per_block,
                max_tokens: args.kv_max_tokens,
            },
        ),
    };
```

Guard: selecting `locus` when the CLI is built without the `locus-kv` feature already errors cleanly inside `from_backend` (Task 6 Step 3), surfaced as an `InferenceError` at session start.

- [ ] **Step 4: Print the allocation count for benchmarking**

In `run.rs`, near the existing `println!("kv_cache_bytes={}", session.kv_cache_bytes());` (run.rs:155), add:

```rust
    #[cfg(feature = "locus-kv")]
    if let Some(allocations) = session.locus_pool_allocation_count() {
        println!("locus_pool_allocation_count={allocations}");
    }
```

- [ ] **Step 5: Verify**

Run: `cargo build -p ferrite-cli --features locus-kv`
Expected: builds.

Run (smoke, no model needed to prove arg parsing rejects bad input): `cargo run -p ferrite-cli --features locus-kv -- --kv-backend bogus 2>&1 | head`
Expected: a clean argument error mentioning `kv-backend`.

- [ ] **Step 6: Commit**

```bash
git add crates/ferrite-cli/Cargo.toml crates/ferrite-cli/src/args.rs crates/ferrite-cli/src/run.rs crates/ferrite-cli/src/benchmark.rs
git commit -m "feat: add --kv-backend CLI flags to select the Locus KV backend"
```

---

### Task 8: Benchmark evidence (allocations, RSS, throughput, correctness)

**Files:**
- Create: `documentation/benchmarks/2026-07-06-locus-kv-backend.md`
- Create: `documentation/dev-notes/2026-07-06-locus-kv-backend.md`

This task produces evidence, not code. Run on the local macOS Apple Silicon host with a real Tier 1 artifact already used by the project (e.g. `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`).

- [ ] **Step 1: Build release with the feature**

```bash
cargo build --release -p ferrite-cli --features locus-kv
```

- [ ] **Step 2: Correctness parity on a real model**

Run the deterministic single-token path both ways and confirm identical token output (mirror the README CLI form; adjust model path/prompt):

```bash
target/release/ferrite --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf --prompt 'hello world' --generate-tokens 1
target/release/ferrite --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf --prompt 'hello world' --generate-tokens 1 --kv-backend locus --kv-tokens-per-block 16 --kv-max-tokens 256
```

Expected: identical generated token id. Record both outputs.

- [ ] **Step 3: Allocation churn**

Run a longer generation with the Locus backend and record `locus_pool_allocation_count` from stdout. Confirm it equals `2 * num_layers * ceil(tokens/tokens_per_block)` (block allocations), i.e. far fewer than `2 * num_layers * tokens` (the Vec backend's per-token `Vec` allocations, which have no counter — state the analytic baseline).

- [ ] **Step 4: Memory (RSS)**

Use the documented memory-sampling flow (README "CLI Memory Sampling") for both backends at a short (32-token) and a longer generation, capturing post-load current RSS and peak RSS via `/usr/bin/time -l`. Record both. The gate: Locus RSS ≤ Vec RSS (no short-sequence regression) and ideally lower at longer lengths.

- [ ] **Step 5: Throughput**

Run `--benchmark-runs` (the existing benchmark path) for both backends, same model/prompt/threads/build. Record tok/s. Gate: Locus within measurement noise of Vec (no loss).

- [ ] **Step 6: Write the benchmark note**

Populate `documentation/benchmarks/2026-07-06-locus-kv-backend.md` with: date, commit/tree state, host + OS, CPU feature flags, model/source/format/quant, prompt, generated-token count, thread count, build mode, and the measured allocation counts, RSS samples, and tok/s for both backends, plus the correctness-parity result. Follow the structure of an existing note in `documentation/benchmarks/`.

- [ ] **Step 7: Write the dev-note**

Populate `documentation/dev-notes/2026-07-06-locus-kv-backend.md`: what changed (the `KvCacheStore` seam + `LocusKvStore`), how it was validated (differential test, fixture parity, real-model parity), the measured wins/no-regressions, what remains unproven (x86_64, server path, longer contexts, NUMA), and the next slice.

- [ ] **Step 8: Commit**

```bash
git add documentation/benchmarks/2026-07-06-locus-kv-backend.md documentation/dev-notes/2026-07-06-locus-kv-backend.md
git commit -m "docs: record Locus KV backend benchmarks and dev-note"
```

---

### Task 9: ADR and Locus proof-of-consumer note

**Files:**
- Create: `documentation/adr/0010-locus-kv-block-backend.md`
- Create (in the Locus repo): `../locus/documentation/dev-notes/2026-07-06-ferrite-consumer.md`

- [ ] **Step 1: Write the ADR**

Create `documentation/adr/0010-locus-kv-block-backend.md` following the existing ADR format (see `documentation/adr/0009-token-prefix-kv-cache.md`). Cover: context (per-token `Vec` churn), decision (the `KvCacheStore` seam + optional Locus mapped block backend, Vec default), the alignment invariant (page-aligned mapped blocks + `bytemuck` = safe zero-copy, no `unsafe` in Ferrite), the new dependency (`locus-alloc` path dep + `bytemuck`, `locus-kv` feature off by default), rejected alternatives (Approach A max-context arena — short-seq RSS regression; Approach C snapshot-only — off-target), consequences, and the deferred scope (concurrent serving/mailbox/NUMA, server opt-in wiring).

- [ ] **Step 2: Add the ADR to the ADR index**

Append a line for ADR 0010 to `documentation/adr/README.md` (match existing entries).

- [ ] **Step 3: Note Ferrite as a Locus consumer**

In the Locus repo, add a short dev-note that Ferrite now embeds `KvBlockPool` as an opt-in KV backend (single-owner, mapped, LIFO), and capture the one API friction found: `KvBlockPool` exposes no immutable/typed block accessor (all reads go through `block_mut(&mut self)`), and `KvBlockTable` does not expose indexed handle access, so the consumer hand-rolls per-(layer,K|V) handle lists. This is backlog for Locus, not a change in this work.

- [ ] **Step 4: Commit (two repos)**

```bash
git add documentation/adr/0010-locus-kv-block-backend.md documentation/adr/README.md
git commit -m "docs: ADR 0010 for the optional Locus KV block backend"
cd ../locus && git add documentation/dev-notes/2026-07-06-ferrite-consumer.md && git commit -m "docs: note Ferrite as a KvBlockPool consumer" && cd -
```

---

## Self-Review

**Spec coverage:**
- `KvCacheStore` seam (Vec default + opt-in Locus): Tasks 2, 3, 6. ✓
- Mapped backing + `bytemuck` zero-copy, no `unsafe`: Task 5. ✓
- Incremental `tokens_per_block` blocks, no short-seq regression: Task 5 (`ensure_block`/`truncate`), Task 8 Step 4. ✓
- Snapshot/prefix-cache format unchanged: Tasks 2/3 (marshal to owned `ScalarLlamaSessionSnapshot`; `runtime.rs` snapshot storage untouched except `?`). ✓
- Feature + flag gating: Tasks 4, 6 (`from_backend`), 7. ✓
- Differential + reference parity tests: Tasks 2, 5, 6; existing suite re-run in Task 3 Step 8. ✓
- Benchmark evidence (allocs/RSS/tok/s): Task 8. ✓
- ADR/dev-note/Locus note: Tasks 8, 9. ✓
- Capacity fixed → `OutOfBlocks` clean error: Task 5 (`new`, `push`), tested Task 5 Step 2. ✓
- Out of scope respected (no mailbox/NUMA/concurrent serving; server opt-in deferred): Tasks note it; server keeps `start_session()`. ✓

**Placeholder scan:** The only intentional "copy from the existing file" is the fixture-model builder in Task 6 Step 8 — unavoidable because the fixture constructor lives in `scalar_reference.rs` and must be reused verbatim; the step names the exact source. CLI arg wiring (Task 7) points at the repo's hand-rolled arg style with a named exemplar (`--generate-tokens`, `Q8KActivationMatvecRole::parse`).

**Type consistency:** Store method names (`layer_len`, `push`, `key`, `value`, `truncate`, `kv_cache_bytes`, `snapshot`, `restore`) are identical across `VecKvStore`, `LocusKvStore`, and the `KvCacheStore` enum. `causal_attention(config, query, store, layer)` matches its one call site. `new_with_options`/`start_session_with_options` return `Result` consistently with all updated callers. `from_backend`/`from_store`/`new_vec` signatures match their call sites. `ScalarLlamaSessionSnapshot::from_layers`/`layers_len`/`layer_keys_owned`/`layer_values_owned` are used consistently by both stores.

## Execution Handoff

(Provided after the plan is approved.)
