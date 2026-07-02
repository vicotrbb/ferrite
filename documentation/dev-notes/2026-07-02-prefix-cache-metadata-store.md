# Prefix Cache Metadata Store

Date: 2026-07-02

## Slice

This slice adds a bounded metadata store for prefix cache entries in
`ferrite-inference`.

The store is part of ADR 0009 phase 1. It still does not own K/V tensors,
perform cache lookup during generation, resume sessions, or change server
behavior.

## Implementation

- Added `PrefixCacheMetadataStore` in
  `crates/ferrite-inference/src/prefix_cache/store.rs`.
- Kept `prefix_cache.rs` focused on public cache identity and entry types.
- The store tracks:
  - maximum entry count;
  - maximum estimated K/V bytes;
  - current estimated K/V bytes;
  - bounded `PrefixCacheEntry` metadata.
- Insert replaces an existing entry with the same key.
- Eviction removes the least-recently-used entry by `last_used_at_tick`, with
  `created_at_tick` as the deterministic tie-breaker.
- `record_hit` updates last-used metadata for future eviction decisions.

Byte accounting uses saturating arithmetic for defensive bookkeeping. The store
returns evicted entries so a future K/V owner can release tensor storage in the
same order as metadata eviction.

## Red Tests

The initial focused test failed before implementation with:

```text
error[E0432]: unresolved import `ferrite_inference::prefix_cache::PrefixCacheMetadataStore`
```

After adding tests for the public store shape, the `is_empty` expectation failed
before the method existed:

```text
error[E0599]: no method named `is_empty` found for struct `PrefixCacheMetadataStore`
```

## Validation

Focused check:

```sh
CARGO_TARGET_DIR=target/codex-prefix-store cargo test -p ferrite-inference --test token_prefix_cache -- --nocapture
```

Result: 6 passed.

Package checks:

```sh
CARGO_TARGET_DIR=target/codex-prefix-store cargo test -p ferrite-inference --tests
cargo fmt --all -- --check
git diff --check
```

Results:

- `cargo test -p ferrite-inference --tests`: passed.
- `cargo fmt --all -- --check`: passed.
- `git diff --check`: passed.

The isolated target directory was removed after validation.

## Limits

This store is still metadata-only. The next implementation slice should connect
real request fingerprints or session-owned K/V handles without widening this
file into a mixed identity, eviction, tensor, and runtime module.
