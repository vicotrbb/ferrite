# Prefix Cache Entry Metadata

Date: 2026-07-02

## Slice

This slice adds metadata-only prefix cache entries in `ferrite-inference`.
It is part of ADR 0009 phase 1 and does not implement cache lookup, K/V state
retention, session snapshot/resume, server wiring, or generation behavior
changes.

## Implementation

- Added `PrefixCacheEntry`.
- The entry records:
  - the exact `PrefixCacheKey`;
  - matched prefix token count, derived from the key;
  - estimated K/V bytes;
  - creation tick;
  - last-used tick.
- Added `record_use` for future eviction-policy bookkeeping.

The tick values are caller-provided counters, not wall-clock timestamps. This
keeps the core inference type deterministic and easy to test.

## Red Test

The initial focused test failed before implementation with:

```text
error[E0432]: unresolved import `ferrite_inference::prefix_cache::PrefixCacheEntry`
```

## Validation

Focused check:

```sh
cargo test -p ferrite-inference --test token_prefix_cache -- --nocapture
```

Result: 4 passed.

Package checks:

```sh
cargo test -p ferrite-inference --tests
cargo fmt --all -- --check
git diff --check
```

Results:

- `cargo test -p ferrite-inference --tests`: passed.
- `cargo fmt --all -- --check`: passed.
- `git diff --check`: passed.

## Limits

This entry type is not a cache store and does not own K/V tensors yet. The next
slice should either derive real request fingerprints in the server/runtime
boundary or add a bounded in-memory metadata store before any session snapshot
or K/V reuse work.
