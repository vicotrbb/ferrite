# 2026-06-27 Scalar Memory Accounting Slice

## Scope

This slice adds explicit byte accounting for the scalar reference path so Tier
0 memory notes can separate file bytes, dequantized scalar weights, and live
K/V cache storage.

## Implementation

- Added `crates/ferrite-inference/src/scalar/memory.rs` for focused memory
  helpers instead of growing the scalar core module.
- Added `ScalarLlamaModel::scalar_weight_bytes()` to count owned F32 scalar
  weight storage.
- Added `ScalarLlamaSession::kv_cache_bytes()` to count live cached K/V F32
  vectors.
- Printed `model_file_bytes`, `scalar_weight_bytes`, and `kv_cache_bytes` from
  the CLI after optional benchmark runs.

## Boundaries

The accounting is intentionally narrow. It does not include tokenizer heap
storage, GGUF parser metadata, vector capacity slack, allocator overhead, code
pages, stack usage, or operating-system page-cache effects. RSS remains the
external process-level memory measurement.

## Evidence

- Red: `cargo test -p ferrite-inference --test scalar_reference
  scalar_model_reports_weight_and_session_kv_cache_bytes` failed because the
  accounting methods did not exist.
- Red: `cargo test -p ferrite-cli --test next_token_cli
  cli_benchmarks_repeated_next_token_runs_after_loading_once` failed because
  the CLI did not print `model_file_bytes`.
- Green: both targeted tests passed after adding the accounting API and CLI
  output.
- Full verification before commit:
  - `cargo fmt --all -- --check`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `git diff --check`
  - hygiene scan reported only the existing `Cargo.toml:16:unsafe_code =
    "forbid"` policy line.
