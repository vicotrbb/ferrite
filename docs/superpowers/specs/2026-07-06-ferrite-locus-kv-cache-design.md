# Design: Optional Locus KV-block backend for Ferrite's session cache

Date: 2026-07-06
Status: Approved design, pending implementation plan
Repos: ferrite (consumer), locus (`locus-alloc`, dependency)

## Goal

Replace the per-token heap-allocation churn in Ferrite's live KV cache with
Locus's `KvBlockPool`, embedded as a single-owner block allocator behind a new
storage abstraction. The current cache stores KV as `Vec<Vec<Vec<f32>>>` and
appends `2 x num_layers` freshly heap-allocated `Vec<f32>` per generated token
on the hot path, with no pooling and substantial per-`Vec` header overhead.

### Success gate

Any measurable win with no regression:

- Improve at least one of {allocations per token, peak/post-load RSS}.
- Zero correctness regression: the Locus backend must produce bit-identical KV
  and logits to the current Vec backend, and must keep matching the existing
  6-prompt llama.cpp deterministic reference.
- No decode-throughput loss (tok/s within measurement noise).

Both allocations and RSS are reported honestly; the benchmark decides which
metric moves. Ferrite's decode time is dominated by the quantized matmul
kernels, so throughput may not move even with a clean integration — that is
acceptable under this gate as long as it does not regress.

## Scope

In scope: the live `ScalarLlamaSession` KV storage (the per-token hot path), a
storage abstraction that lets the proven Vec path and the Locus path coexist,
opt-in gating, differential + reference-parity tests, and benchmark evidence.

Out of scope (YAGNI, explicit future work):

- Concurrent / parallel generation, `ChunkMailbox` remote-free, and the
  owner-drain pattern. Ferrite keeps its single inference permit
  (`INFERENCE_PERMITS = 1`); the mailbox has no consumer until Ferrite serves
  generations in parallel.
- NUMA binding (`bind_to_node`, `mbind`) and any Linux-only path. macOS /
  Apple Silicon is the development and measurement host.
- Pooling the prefix-cache snapshot storage (the deep-cloned owned snapshots).
  That is a separate, optional, lower-risk slice ("Approach C") and is left for
  later. This design leaves the snapshot format unchanged.
- Modifying Locus's public API. Locus is consumed as-is. API friction is
  captured as backlog, not fixed here, unless a hard blocker appears.

## Background: current state

- KV cache: `ScalarLlamaSession { layer_keys: Vec<Vec<Vec<f32>>>,
  layer_values: Vec<Vec<Vec<f32>>>, cached_token_count, .. }`
  (`crates/ferrite-inference/src/scalar/session.rs`). Outer = layer, middle =
  token position, inner = `head_kv_count * head_dim` f32.
- Growth: `layer_keys[l].push(key)` / `layer_values[l].push(value)` per layer
  per accepted token (`session.rs`, `accept_token_inner`). No preallocation.
- Read: `causal_attention(config, query, keys_by_position: &[Vec<f32>],
  values_by_position: &[Vec<f32>])` (`scalar/attention.rs`) — sequential,
  single-threaded, one position at a time (K-score phase, then V-output phase).
- Truncate/reset: `Vec::truncate` (`scalar/session/cache.rs`); snapshot
  save/restore deep-clones (`scalar/session/snapshot.rs`).
- Prefix cache: server-side `RuntimePrefixCache` stores full deep-cloned
  `ScalarLlamaSessionSnapshot` values (owned `Vec<Vec<Vec<f32>>>`), keyed by
  token identity, LRU-evicted by a byte budget. Inference-crate prefix cache is
  metadata only.
- Byte accounting: `memory::kv_cache_bytes(&[Vec<Vec<f32>>], ..)` sums logical
  `f32` bytes only; it ignores `Vec` capacity slack and per-`Vec` header
  overhead.
- Concurrency: single inference permit; attention/KV are single-threaded;
  rayon is used only inside the matmul kernels, never around the cache.
- Abstraction: none. Storage is concrete inline `Vec` fields. The only natural
  seams are the attention read slice type and the `kv_cache_bytes` free
  function.

## Background: Locus contract (as consumed)

- Dependency: `locus-alloc` (path dep to `../locus/crates/locus-alloc`), sole
  runtime dep `libc`. `numa` feature is Linux-only and NOT enabled.
  Crate-wide `deny(unsafe_code)` except its `sys` module.
- `KvBlockPool::new_mapped(home_node: NodeId, block_size: usize,
  capacity: usize, reuse_order: KvReuseOrder) -> Result<Self, KvBlockPoolError>`
  — one contiguous mmap region, blocks at fixed offsets. Works on macOS.
- Hot-path methods take `&mut self`: `allocate() -> Result<KvBlockHandle, _>`,
  `block_mut(handle) -> Result<&mut [u8], _>` (the only way to reach block
  bytes; even reads need `&mut self`), `free(handle) -> Result<(), _>`,
  `stats() -> KvBlockPoolStats` (includes `allocation_count`, `free_count`,
  `high_water_mark`).
- `KvBlockHandle` is opaque `Copy` plain data (index + generation, both
  private). Generation-checked: double-free / use-after-free / stale handle
  return `Err(InvalidHandle)` rather than corrupting memory — but the consumer
  must propagate those errors.
- A "block" is a fixed-size untyped `&mut [u8]`. Size and capacity are fixed at
  pool creation; the pool never grows. `allocate` returns `OutOfBlocks` when
  exhausted. Locus imposes no tensor layout; the consumer writes its own bytes.
- `KvBlockTable::new(sequence_id, tokens_per_block: u16)` +
  `append_tokens(pool, token_count)` / `release_all(pool)` is the intended
  token->block helper. Use it if it exposes indexed handle access; otherwise a
  thin hand-rolled equivalent (an ordered `Vec<KvBlockHandle>`) is acceptable.
- Single-owner: `allocate`/`free`/`block_mut` need `&mut self`; the pool is
  never shared for mutation. `block_mut` borrows the pool mutably, so only one
  block slice can be live at a time.
- Alignment: mmap base is page-aligned; therefore mapped blocks are >=4-byte
  aligned. This is the invariant that makes safe zero-copy `f32` views possible.

## Architecture

### The `KvCacheStore` seam

Introduce one boundary all KV storage goes through, as an enum (no `dyn`, no
vtable on the hot path):

```rust
enum KvCacheStore {
    Vec(VecKvStore),      // wraps today's Vec<Vec<Vec<f32>>>; the default
    Locus(LocusKvStore),  // Locus-backed; compiled only under feature `locus-kv`
}
```

`ScalarLlamaSession` replaces its two `Vec<Vec<Vec<f32>>>` fields with a single
`store: KvCacheStore`. All session and attention code calls store methods. The
Vec variant is byte-for-byte today's behavior and remains the default.

Proposed store interface (final method set finalized in the plan):

```rust
fn layer_count(&self) -> usize;
fn cached_token_count(&self) -> usize;
fn push(&mut self, layer: usize, key: &[f32], value: &[f32]);
fn commit_token(&mut self);            // advance token count after all layers pushed
fn with_layer_keys(&mut self, layer, f: impl FnMut(usize /*pos*/, &[f32]));
fn with_layer_values(&mut self, layer, f: impl FnMut(usize /*pos*/, &[f32]));
fn truncate(&mut self, token_count: usize);
fn kv_cache_bytes(&self) -> u128;
fn snapshot(&self) -> ScalarLlamaSessionSnapshot;   // owned Vec form, unchanged
fn restore(&mut self, snapshot: &ScalarLlamaSessionSnapshot);
```

The read accessors take a callback (or an equivalent block-walking iterator)
rather than returning a borrowed slice, so the Locus variant can hold its
single `&mut` block borrow for the duration of one block's worth of positions
and release it before the next block. This matches attention's access pattern
(K-score phase fully, then V-output phase) and keeps a single pool sufficient.

### `LocusKvStore` internals

- One `KvBlockPool` per session, `new_mapped(NodeId(0), block_size, capacity,
  KvReuseOrder::Lifo)`. Mapped backing is deliberate: page alignment enables
  safe zero-copy `f32` views.
- Element view without `unsafe`: use `bytemuck::cast_slice` /
  `cast_slice_mut` on the aligned block bytes. Ferrite adds `bytemuck` as a
  dependency (safe API; the `unsafe` lives inside `bytemuck`). Invariant:
  block base is page-aligned; all intra-block token offsets are multiples of
  `head_kv_dim * 4` bytes, hence 4-aligned; block byte length is a multiple of
  4. These guarantees are asserted and documented.
- Block layout: `head_kv_dim = head_kv_count * head_dim` (f32 count per token
  per K or V). `block_size = tokens_per_block * head_kv_dim * 4` bytes. Per
  `(layer, K|V)` keep an ordered list of block handles. Position `t` maps to
  block `t / tokens_per_block`, float offset `(t % tokens_per_block) *
  head_kv_dim`.
- Push: allocate a new block only when `t % tokens_per_block == 0`; write the
  K and V float slices into the current block at the offset. With
  `tokens_per_block = 16`, ~15 of every 16 tokens allocate nothing, versus
  `2 * num_layers` allocations every token today.
- Read: attention walks block-by-block — one `block_mut` per block amortizes
  the generation check over `tokens_per_block` positions, and tokens are
  contiguous within a block (better locality than the current per-token
  pointer chase). A single pool suffices because K and V phases are sequential.
- Capacity fixed at creation: `2 * num_layers * ceil(max_tokens /
  tokens_per_block) + headroom`, sized to `hard-max-tokens`. `OutOfBlocks` is a
  clean error, consistent with Ferrite's existing hard token cap.
- Lifecycle: on session drop, blocks free back to the pool; LIFO reuse warms
  the next session (the cross-request churn win). The pool is owned by the
  store, which is owned by the session — single owner, no sharing.

### Data flow

Push order, causal masking, RoPE, and the dot-product kernels are unchanged;
they still receive `&[f32]` for query/key/value. Only the origin of the key/
value slices changes (a block view instead of a `Vec`). Snapshot copies blocks
into the owned `ScalarLlamaSessionSnapshot`; restore copies the owned form into
freshly allocated blocks. The prefix cache, its byte budget, and its LRU
eviction are untouched.

### Byte accounting

`kv_cache_bytes()` for the Locus variant reports true block bytes and can also
surface honest overhead (unlike the current count, which ignores `Vec` slack).
For the prefix-cache byte budget, the reported value must remain comparable to
the existing logical-byte figure so eviction behavior does not silently change;
any intended difference is documented.

## Config and feature gating

- Cargo feature `locus-kv` on `ferrite-inference` pulls in the `locus-alloc`
  path dependency and `bytemuck`. Feature off = zero new dependencies, only
  `VecKvStore` compiles, no behavior change. The feature is surfaced up to
  `ferrite-server` and `ferrite-cli`.
- Runtime selection via a `ScalarExecutionOptions` field
  (`kv_store_backend`, default `Vec`), exposed as a CLI flag
  (`--kv-backend locus [--kv-tokens-per-block 16]`) and a server config field.
  Opt-in only, for A/B benchmarking, until the evidence justifies flipping the
  default.

## Testing (correctness is the hard gate)

- Differential test: for random token streams, `VecKvStore` and `LocusKvStore`
  yield bit-identical key and value bytes at every `(layer, position)` (compared
  through the same read accessor), identical snapshot round-trips, and identical
  truncation results.
- Reference parity: re-run the existing 6-prompt llama.cpp deterministic gate
  with `--kv-backend locus`; logits and token IDs must be identical. Swapping
  storage cannot change output.
- Unit/behavior tests: truncation, snapshot save/restore, `OutOfBlocks` error
  handling, alignment-invariant assertions, and capacity sizing.
- The Locus path must uphold `deny(unsafe_code)` in Ferrite code (all low-level
  work goes through `bytemuck` and Locus).

## Evidence / benchmarks

Reuse Ferrite's benchmark/profile CLI and RSS sampling (`--benchmark-runs`,
`--sleep-after-load-ms` + `ps`/`/usr/bin/time -l`, `kv_cache_bytes`) to compare
Vec vs Locus at short (32-token) and longer sequences on:

- allocations per token (Locus `stats().allocation_count`; optionally a
  counting global allocator for the Vec path),
- post-load and peak RSS,
- `kv_cache_bytes`,
- decode throughput (tok/s).

Record a dev-note and a `documentation/benchmarks/` note per the operating
model, with host, model, quantization, prompt, token counts, thread count,
build mode, and commit/tree state. Report honestly which metric moved. Win =
allocs and/or RSS improve, tok/s within noise, correctness identical.

## Documentation / ADRs

- Ferrite: a dev-note for the slice, a benchmark note, and an ADR ("KV storage
  abstraction and optional Locus block-pool backend") — a durable architecture
  boundary plus a new dependency with a documented alignment invariant.
- Locus: a short note that Ferrite is now a real proof-of-consumer, plus any
  API friction as backlog (for example, no immutable or typed block accessor;
  all block access funnels through `block_mut(&mut self)`). Changing Locus is
  out of scope for this slice.

## Risks and mitigations

- Attention read rewrite touches the correctness-proven path. Mitigation:
  the differential test and the 6-prompt reference gate must both stay green;
  the Vec path remains the default and fallback.
- Throughput regression from per-access handle validation. Mitigation:
  block-by-block reads amortize validation over `tokens_per_block`; measure
  before claiming; escalate only if the benchmark shows a loss.
- Alignment / `unsafe`-free reinterpretation. Mitigation: mapped (page-aligned)
  backing plus `bytemuck`; assert the invariants.
- Fixed pool capacity. Mitigation: size to `hard-max-tokens`; treat
  `OutOfBlocks` as a clean, tested error.
- Short-sequence memory regression (the reason Approach A was rejected).
  Mitigation: incremental `tokens_per_block` allocation so memory scales with
  actual sequence length.

## Open items for the implementation plan

- Confirm whether `KvBlockTable` exposes indexed handle access; if not, use a
  thin hand-rolled ordered `Vec<KvBlockHandle>` per `(layer, K|V)`.
- Finalize the exact store trait/enum method set and the attention accessor
  shape (callback vs block iterator).
- Decide the default `tokens_per_block` to benchmark first (start at 16).
- Decide how the prefix-cache byte budget reads Locus-backed bytes without
  changing eviction behavior.
