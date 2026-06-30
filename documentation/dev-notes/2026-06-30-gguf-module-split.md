# GGUF Module Split

Date: 2026-06-30

## Scope

This slice splits the `ferrite-model` GGUF parser implementation into smaller
focused modules without changing the public `ferrite_model::gguf::*` API.

The refactor keeps `crates/ferrite-model/src/gguf.rs` as the public facade and
parser orchestration module, then moves:

- GGUF metadata and tensor type definitions to
  `crates/ferrite-model/src/gguf/types.rs`
- Byte-reader internals and raw tensor-info parsing to
  `crates/ferrite-model/src/gguf/reader.rs`

This reduces the main GGUF module from 680 lines to 291 lines and keeps the new
submodules below 220 lines each.

## Verification

Commands:

```sh
cargo test -p ferrite-model -- --nocapture
cargo fmt --all -- --check
git diff --check
CARGO_BUILD_JOBS=2 cargo clippy --workspace --all-targets -- -D warnings
```

Observed results:

- `cargo test -p ferrite-model -- --nocapture`: 6 GGUF reader tests passed, 6
  tokenizer metadata tests passed, and doc tests had 0 tests.
- `cargo fmt --all -- --check`: passed.
- `git diff --check`: passed.
- `CARGO_BUILD_JOBS=2 cargo clippy --workspace --all-targets -- -D warnings`:
  passed in 3m 25s.

## Result

The GGUF parser is organized into clearer Rust module boundaries while
preserving parser behavior and downstream crate compatibility.

## Limits

This is a structure-only refactor. It does not add new GGUF metadata coverage,
new tensor formats, mmap behavior, or parser fuzz/property tests.
