# 2026-06-27 CLI Drop Model Bytes Slice

## Scope

This slice stops retaining the raw GGUF file byte buffer in the CLI after the
scalar model and tokenizer have been constructed.

## Implementation

- Captured `model_file_bytes` immediately after reading the model file.
- Built the owned GGUF metadata, tokenizer, and scalar model from the raw byte
  buffer.
- Dropped the raw byte buffer before prompt tokenization and inference.
- Added `model_file_retained_bytes=0` to CLI memory output.

## Boundaries

This does not eliminate the transient load-time overlap between the raw GGUF
buffer and dequantized F32 scalar weights. It only removes the raw model byte
buffer from the retained post-load CLI state. Process peak-memory tools may
still report load-time peaks.

## Evidence

- Red: `cargo test -p ferrite-cli --test next_token_cli
  cli_benchmarks_repeated_next_token_runs_after_loading_once` failed because
  `model_file_retained_bytes=0` was missing.
- Green: the same targeted test passed after dropping the raw bytes and
  printing retained-byte accounting.
- Full verification before commit:
  - `cargo fmt --all -- --check`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `git diff --check`
  - hygiene scan reported only the existing `Cargo.toml:16:unsafe_code =
    "forbid"` policy line.
