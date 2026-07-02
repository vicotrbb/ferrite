# Scalar Session Cache Snapshot

Date: 2026-07-02

## Slice

This slice adds an inference-owned scalar session cache snapshot API.

It is ADR 0009 phase 2 groundwork: a future prefix cache can retain K/V state
without moving K/V ownership into the OpenAI server layer.

This slice does not add cache lookup, cache insertion, cache eviction of K/V
state, OpenAI `cached_tokens` changes, or cross-request reuse.

## Implementation

- Added `crates/ferrite-inference/src/scalar/session/snapshot.rs`.
- Added `ScalarLlamaSessionSnapshot` as an owned clone of scalar per-layer K/V
  state plus cached token count.
- Added `ScalarLlamaSession::cache_snapshot()`.
- Added `ScalarLlamaSession::restore_cache_snapshot()`.
- Re-exported `ScalarLlamaSessionSnapshot` from the scalar module.
- Kept the snapshot fields private so external callers can inspect token count
  and bytes without constructing arbitrary K/V state.

## Red Test

The focused test was written before implementation:

```sh
CARGO_TARGET_DIR=target/codex-session-snapshot cargo test -p ferrite-inference --test scalar_session_cache session_restores_cache_snapshot_to_resume_prefix -- --nocapture
```

It failed for the expected missing API:

```text
error[E0599]: no method named `cache_snapshot` found for struct `ScalarLlamaSession<'a>`
error[E0599]: no method named `restore_cache_snapshot` found for struct `ScalarLlamaSession<'a>`
```

## Validation

Focused check after implementation:

```sh
CARGO_TARGET_DIR=target/codex-session-snapshot cargo test -p ferrite-inference --test scalar_session_cache session_restores_cache_snapshot_to_resume_prefix -- --nocapture
```

Result: 1 passed.

Inference package tests:

```sh
CARGO_TARGET_DIR=target/codex-session-snapshot cargo test -p ferrite-inference --tests
```

Result:

- `ferrite-inference` unit tests: 88 passed.
- `matvec_kernel_check`: 14 passed.
- `scalar_profile`: 1 passed.
- `scalar_reference`: 30 passed.
- `scalar_session_cache`: 3 passed.
- `token_prefix_cache`: 6 passed.

Formatting and whitespace checks:

```sh
cargo fmt --all -- --check
git diff --check
```

Results:

- `cargo fmt --all -- --check`: passed.
- `git diff --check`: passed.

## Limits

The snapshot currently clones scalar K/V vectors. That is correct for a
reference implementation and useful for exact-prefix correctness tests, but it
is not a final memory-efficient cache representation.

The restore API trusts the caller to pair the snapshot with the correct
model/cache key. It performs structural layer and token-count checks, but model
identity and tokenizer/template/execution fingerprints still belong in the
prefix cache key path before any cross-request reuse is enabled.

No real model benchmark was run in this slice.
