# ADR 0010: Locus KV block-pool backend

Date: 2026-07-06

Status: Accepted as an optional backend

## Context

The default session KV store grows nested vectors and can allocate new key and
value buffers for every layer and token. Fixed-size block pooling can reduce
allocation churn, but it must not change token output, silently exceed a memory
budget, or add an unsafe public boundary.

## Decision

`ScalarLlamaSession` stores KV state behind a static `KvCacheStore` enum:

- `VecKvStore` preserves the default nested-vector behavior;
- `LocusKvStore` uses a per-session `locus_alloc::KvBlockPool`;
- the enum avoids dynamic dispatch on the hot path.

The Locus backend is opt-in twice. It requires the `locus-kv` Cargo feature and
an explicit runtime `KvBackend::Locus` selection. The CLI exposes this as
`--kv-backend locus`, `--kv-tokens-per-block`, and `--kv-max-tokens`.

Pool capacity is fixed from layer count, tokens per block, and maximum tokens.
Crossing capacity returns an error. Truncation returns no-longer-needed blocks
to the per-session pool in LIFO order. The pool is not global and does not
reuse blocks across sessions.

Ferrite uses safe `bytemuck` casts for block bytes. Mapped block bases, block
sizes, and token offsets are all aligned to `f32`; cast failures still return an
error so a future invariant regression fails safely.

The backend must produce bit-identical logits, token IDs, snapshots, and KV
byte accounting compared with the default store.

## Consequences

Default builds do not include the optional allocator dependencies or Locus
code. The server currently uses the default backend, so Locus remains a local
CLI and inference experiment.

Pooling can reduce allocation count but reserves fixed capacity for a session.
Real-model RSS and throughput must be measured before expanding its default or
server scope.

## Alternatives considered

- **One maximum-context arena per layer.** Rejected because short sessions
  would reserve capacity for the longest configured context.
- **Global cross-session pool.** Deferred because ownership, concurrency, NUMA,
  and lifecycle semantics require a separate design.
- **Cache only server snapshots.** Rejected for this decision because it does
  not address live per-token allocation churn.

## Evidence

- `crates/ferrite-inference/src/scalar/kv_store.rs` defines the static backend
  boundary.
- `crates/ferrite-inference/src/scalar/kv_store/locus.rs` tests block reuse,
  truncation, capacity failure, and snapshot round trips.
- `crates/ferrite-inference/tests/kv_store_backend_parity.rs` proves output
  parity across a block boundary.
- [`../benchmarks/2026-07-06-locus-kv-backend.md`](../benchmarks/2026-07-06-locus-kv-backend.md)
  records the accepted and still-pending evidence.
