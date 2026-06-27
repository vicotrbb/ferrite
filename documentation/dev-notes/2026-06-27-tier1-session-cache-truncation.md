# 2026-06-27 Tier 1 Session Cache Truncation

## Slice

Tier 1 includes a KV-cache gate: cache state must grow and shrink correctly
across turns. Ferrite already grew the scalar session cache as tokens were
accepted, but there was no public way to trim cached positions.

This slice adds `ScalarLlamaSession::truncate_cache(token_count)`, which:

- rejects attempts to extend the cache through the truncation API;
- truncates key and value vectors for every layer to the requested token count;
- updates `cached_token_count`; and
- keeps subsequent token acceptance equivalent to recomputing from the retained
  prefix.

The behavior is covered in
`crates/ferrite-inference/tests/scalar_session_cache.rs`.

## Validation

Test-first failure before implementation:

```text
cargo test -p ferrite-inference --test scalar_session_cache -- --nocapture
```

The new tests failed because `ScalarLlamaSession::truncate_cache` did not exist.

Passing checks after implementation:

```text
cargo test -p ferrite-inference --test scalar_session_cache -- --nocapture
cargo test -p ferrite-inference --test scalar_reference
```

The new cache truncation target passed both tests, and all 16 scalar reference
integration tests still passed.

## Remaining Work

This proves scalar in-memory cache truncation. It does not yet implement
sliding-window eviction, cache compaction, quantized cache storage, or
multi-turn chat policies for larger Tier 1 models.
