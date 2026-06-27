# 2026-06-27 Scalar Session Cache Slice

## Scope

This slice adds an incremental scalar session boundary that retains per-layer
K/V cache entries across accepted tokens.

## Implementation

- Added `ScalarLlamaSession` in a focused `scalar/session.rs` module.
- Added `ScalarLlamaModel::start_session`.
- Rewired `next_token_for_prompt` to use a fresh session, keeping the full
  prompt path aligned with the incremental path.
- Exposed `cached_token_count` for deterministic tests and benchmark harnesses.

## Boundaries

This is still scalar reference execution. It does not add production serving
state, sampling, multi-token generation loops, cache eviction, cache
compression, or optimized KV layouts.

## Evidence

- Red: `cargo test -p ferrite-inference --test scalar_reference
  scalar_session_reuses_cached_prompt_state_incrementally` failed because
  `ScalarLlamaModel::start_session` did not exist.
- Green: the same targeted test passed after adding the session module.
- The test proves the session result matches the existing full-prompt scalar
  path for the prompt, then accepts the generated token incrementally and
  matches the full extended prompt while preserving cached token count.
