# ADR 0010: Locus KV Block-Pool Backend

Date: 2026-07-06

Status: Accepted

## Context

Ferrite's live scalar KV cache stores per-layer key/value state as
`ScalarLlamaSession { layer_keys: Vec<Vec<Vec<f32>>>, layer_values:
Vec<Vec<Vec<f32>>>, .. }` (`crates/ferrite-inference/src/scalar/session.rs`).
Outer index is layer, middle index is token position, inner `Vec<f32>` is one
K or V vector (`head_kv_count * head_dim` floats). On the hot path, every
accepted token calls `layer_keys[l].push(key)` / `layer_values[l].push(value)`
for every layer (`VecKvStore::push` in
`crates/ferrite-inference/src/scalar/kv_store.rs`, formerly inline on
`session.rs`): that is `2 * num_layers` freshly heap-allocated `Vec<f32>`
values per generated token, with no pooling or reuse, plus the header/capacity
overhead each nested `Vec` carries. There was no storage abstraction; the only
seams were the attention read-slice type and the `kv_cache_bytes` free
function.

Locus (`locus-alloc`) provides `KvBlockPool`, a single-owner, fixed-capacity
block allocator purpose-built for exactly this shape of churn: fixed-size
blocks, mapped (page-aligned) backing, generation-checked handles, and LIFO
reuse. `docs/superpowers/specs/2026-07-06-ferrite-locus-kv-cache-design.md`
records the full design and its success gate (no correctness regression, no
decode-throughput loss, and an improvement in at least one of {allocations per
token, peak/post-load RSS}).

## Decision

Introduce a `KvCacheStore` seam that all KV storage goes through, as an enum
(no `dyn`, no vtable on the hot path):

```rust
enum KvCacheStore {
    Vec(VecKvStore),      // wraps the historical Vec<Vec<Vec<f32>>>; the default
    Locus(LocusKvStore),  // Locus-backed; compiled only under feature `locus-kv`
}
```

(`crates/ferrite-inference/src/scalar/kv_store.rs`). `ScalarLlamaSession`
holds a single `store: KvCacheStore` field instead of the two raw `Vec` fields.
`VecKvStore` reproduces the historical nested-`Vec` behavior exactly and is
the compiled-in default when the `locus-kv` feature is off. `LocusKvStore`
(`crates/ferrite-inference/src/scalar/kv_store/locus.rs`) wraps one
`locus_alloc::KvBlockPool` per session (`KvBlockPool::new_mapped(NodeId(0),
block_size, capacity, KvReuseOrder::Lifo)`), with one ordered
`Vec<KvBlockHandle>` per `(layer, K|V)`. Each block holds `tokens_per_block`
token positions; a new block is allocated only when a push crosses a block
boundary (`ensure_block`), so ~`tokens_per_block - 1` of every
`tokens_per_block` pushes allocate nothing. `truncate` frees blocks back to
the pool down to the number needed for the retained token count
(`pool_stats().allocated` strictly decreases). Push order, causal masking,
RoPE, and the dot-product kernels are unchanged — only the origin of the
key/value slices changes, from an owned `Vec<f32>` to a block-backed `&[f32]`
view.

The backend is opt-in at two independent levels, both required:

- Compile time: the `locus-kv` Cargo feature on `ferrite-inference`, threaded
  through `ferrite-cli`'s own `locus-kv` feature. Off by default.
- Run time: an explicit `KvBackend::Locus { tokens_per_block, max_tokens }`
  selection (`KvCacheStore::from_backend`), surfaced as CLI flags
  (`--kv-backend locus --kv-tokens-per-block <n> --kv-max-tokens <n>`,
  default `tokens_per_block = 16`). `KvBackend::Vec` remains the default.

Requesting the Locus backend without the feature compiled in returns a clean
`InferenceError` rather than a compile error or silent fallback
(`KvCacheStore::from_backend`'s `#[cfg(not(feature = "locus-kv"))]` arm).

Output must be, and is, bit-identical between backends: same `token_id`,
`logits`, and `kv_cache_bytes` for the same input, proven by a differential
parity test (see Evidence). Swapping storage does not change model output.

## Alignment and the unsafe boundary

Ferrite's workspace denies unsafe code (`deny(unsafe_code)`), and this design
adds no exception. `LocusKvStore` reinterprets pool block bytes (`&mut [u8]`,
the only type `KvBlockPool::block_mut` returns) as `&[f32]` / `&mut [f32]`
using `bytemuck::try_cast_slice` / `try_cast_slice_mut` — a safe API; the
`unsafe` needed to reinterpret the bytes lives inside `bytemuck`, not in
Ferrite.

This is sound only because of an alignment invariant, which this decision
states and which the code relies on:

- `KvBlockPool::new_mapped` backs the pool with one contiguous `mmap` region,
  and `mmap` returns page-aligned memory. Every block therefore starts at an
  address that is at least 4-byte aligned (page alignment is always a
  multiple of 4).
- `block_size = tokens_per_block * head_kv_dim * size_of::<f32>()` bytes is
  constructed as a multiple of 4 by construction (`LocusKvStore::new`).
- Every intra-block token offset is `(position % tokens_per_block) *
  head_kv_dim * size_of::<f32>()` (`LocusKvStore::byte_range`) — also a
  multiple of 4.

Because both the block base address and every offset into it are 4-aligned,
`try_cast_slice[_mut]::<u8, f32>` on any `[start..end]` sub-slice succeeds
(returns `Ok`, never the `PodCastError` path) for every read and write this
code performs. `write_block` and `read_block` still handle the `Err` case
explicitly rather than assuming it can't happen, so a future change that
breaks this invariant fails as a clean `InferenceError`, not a panic or UB.

## Dependencies

`locus-alloc` (path dependency on `../../../locus/crates/locus-alloc`) and
`bytemuck` are both added as `optional = true` dependencies of
`ferrite-inference`, gated behind one feature:

```toml
locus-alloc = { path = "../../../locus/crates/locus-alloc", optional = true }
bytemuck = { version = "1", optional = true }

[features]
locus-kv = ["dep:locus-alloc", "dep:bytemuck"]
```

`ferrite-cli` exposes its own `locus-kv` feature that forwards to
`ferrite-inference/locus-kv`. With the feature off (the default), neither
dependency is pulled in, only `VecKvStore` compiles, and behavior and code
path are byte-for-byte the pre-existing Vec-only path. Locus's `numa` feature
(Linux-only) is **not** enabled by this integration.

## Rejected alternatives

- **One max-context arena block per layer (Approach A).** Allocate a single
  block sized for the hard maximum context length per `(layer, K|V)` up
  front. Rejected because it over-commits memory for short sequences: a
  session generating a handful of tokens would still reserve and (depending
  on backing) potentially fault in memory for the full configured max
  context, causing a short-sequence RSS regression relative to today's
  Vec path, which only grows with actual token count. The chosen design
  instead allocates incrementally in `tokens_per_block`-sized chunks, so
  memory scales with the sequence actually generated.
- **Pool only the prefix-cache snapshot storage (Approach C).** Apply Locus
  to the server-side `RuntimePrefixCache`'s deep-cloned owned snapshots
  instead of the live per-token session cache. Rejected as off-target for
  this slice: it does not touch the per-token heap-allocation churn on the
  hot decode path, which is the problem this design set out to solve. It
  remains a separate, optional, lower-risk slice for later; this design
  leaves the snapshot format (`ScalarLlamaSessionSnapshot`, owned
  `Vec<Vec<Vec<f32>>>`) unchanged.

## Consequences

- The Locus backend is strictly opt-in and additive: with the feature and
  flag both off (today's default in every existing binary and deployment),
  there is zero behavior change, zero new dependencies, and zero new code
  compiled in.
- `ferrite-server` has no `locus-kv` feature, no `--kv-backend` flag, and no
  `KvBackend` reference anywhere in `crates/ferrite-server/src` today. The
  server continues to construct sessions via `start_session()` with no
  Locus wiring at all. Wiring an equivalent opt-in path into the server is
  explicit future work, contingent on real-model RSS/throughput evidence,
  not part of this decision.
- Concurrent/parallel generation, `ChunkMailbox` remote-free, the
  owner-drain pattern, and NUMA binding (`bind_to_node`, `mbind`) are out of
  scope. Ferrite keeps a single inference permit (`INFERENCE_PERMITS = 1`);
  the mailbox and NUMA placement have no consumer until Ferrite serves
  generations in parallel, and this host set is Apple Silicon / macOS only.
- Pool capacity is fixed at session-store creation
  (`2 * num_layers * ceil(max_tokens / tokens_per_block)` blocks); exceeding
  it returns a clean `OutOfBlocks`-derived `InferenceError` rather than
  growing or corrupting memory, consistent with Ferrite's existing hard
  token cap.
- Real-model evidence for the design's actual success-gate metrics — peak
  and post-load RSS, decode throughput, and allocation churn over a longer
  generation on a Tier 1 model — is pending a Tier 1 GGUF artifact (none is
  present on the development host and none was downloaded for this slice).
  What is proven today is bit-identical correctness, pool mechanics (unit
  tests), a real measured allocation count on a synthetic fixture matching
  the analytic formula exactly, and a clean release build with the feature.
  See `documentation/benchmarks/2026-07-06-locus-kv-backend.md` for the
  full proven/pending breakdown and exact repro commands.

## Evidence

- `crates/ferrite-inference/tests/kv_store_backend_parity.rs` —
  `locus_backend_matches_vec_logits`: runs the same 5-token prompt (spanning
  a block boundary) through the Vec and Locus backends against a fixture
  GGUF whose Q/K/V/O and FFN weights genuinely mix across positions, and
  asserts bit-identical `token_id`, `logits`, and `kv_cache_bytes`.
- `crates/ferrite-inference/src/scalar/kv_store/locus.rs` unit tests
  (`cargo test -p ferrite-inference --features locus-kv --lib
  scalar::kv_store::locus`): round-tripping across multiple block
  boundaries, block reclamation on truncate (`pool_stats().allocated`
  strictly decreases), out-of-blocks error handling, and snapshot
  round-trip via re-push.
- `documentation/benchmarks/2026-07-06-locus-kv-backend.md` — full evidence
  note: release build with the feature, both test suites' pasted output, an
  end-to-end CLI run on a synthetic fixture (`--kv-backend locus
  --kv-tokens-per-block 2` producing identical `next_token_id`,
  `generated_token_ids`, and `kv_cache_bytes` to the Vec run, plus a real
  measured `locus_pool_allocation_count=8` matching the analytic formula
  `2 * num_layers * ceil(tokens / tokens_per_block)`), and an explicit
  PROVEN/PENDING split for the real-model RSS/throughput numbers the design's
  success gate ultimately depends on.
- `documentation/dev-notes/2026-07-06-locus-kv-backend.md` — narrative dev
  note for this slice, including the CLI/feature surface and the exact scope
  boundary (server has no wiring yet).
- `docs/superpowers/specs/2026-07-06-ferrite-locus-kv-cache-design.md` — the
  approved design this decision implements, including the rejected
  alternatives above and the out-of-scope list.
