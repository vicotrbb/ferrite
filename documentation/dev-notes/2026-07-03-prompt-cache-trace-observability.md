# Prompt Cache Trace Observability

Date: 2026-07-03

## Goal

Add opt-in prompt-cache trace output so long-chat proofs can explain cache reuse
without inspecting generated text manually.

## Context

This implements the first experiment from
`documentation/theories/2026-07-03-qwen-0-5b-cache-stability.md`.
The Qwen 0.5B full-matrix proof passed, but generated follow-up rows showed
large TTFT swings that tracked cached prompt-token depth.

## Changes

- Added a focused runtime cache-trace model in
  `crates/ferrite-server/src/runtime/cache_trace.rs`.
- Preserved default OpenAI-compatible response shape unless clients opt in with
  `metadata.ferrite_cache_trace = "true"`.
- Serialized the trace under
  `usage.prompt_tokens_details.ferrite_cache`.
- Added `--prompt-cache-trace` to the throughput client and long-chat gate.
- Printed lookup, prompt hash, selected entry hash, and shared-prefix depth in
  streaming and long-chat proof output.
- Tightened runtime cache lookup so hit traces require a usable cache value, not
  only metadata.

## Validation

Commands run:

```sh
cargo fmt
cargo fmt -- --check
cargo test -p ferrite-server --lib -- --nocapture
cargo test -p ferrite-server --test long_chat_gate -- --nocapture
cargo clippy -p ferrite-server --all-targets -- -D warnings
git diff --check
```

Results:

- `cargo fmt -- --check`: passed.
- `cargo test -p ferrite-server --lib -- --nocapture`: 384 passed.
- `cargo test -p ferrite-server --test long_chat_gate -- --nocapture`: 50 passed.
- `cargo clippy -p ferrite-server --all-targets -- -D warnings`: passed.
- `git diff --check`: passed.

## Results

The implementation does not optimize cache behavior yet. It makes the next
real-model reruns explainable by exposing token-level prompt identity,
cache-lookup classification, selected-entry identity, and shared-prefix depth.

## Follow-Ups

- Rerun the 128-token diagnostic gate with `--prompt-cache-trace`.
- Rerun the Qwen 0.5B 1024-token lane with the trace enabled.
- Compare `usage.prompt_tokens_details.cached_tokens` against
  `ferrite_cache.shared_prefix_tokens` in every generated follow-up row.
- Use trace output to decide whether the next optimization is prompt rendering,
  cache namespace/key stability, eviction policy, or generated-context shaping.
