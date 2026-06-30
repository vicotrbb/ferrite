# Scalar Fixture Tensor Module Split

Date: 2026-06-30

## Scope

This slice splits synthetic quantized scalar Llama tensor-table builders out of
`crates/ferrite-fixtures/src/scalar_llama.rs` into a private fixture module:

- `crates/ferrite-fixtures/src/scalar_llama_tensors.rs`

The public fixture API exported by `ferrite_fixtures` is unchanged. The
`scalar_llama.rs` module continues to own public GGUF fixture construction,
while the new private module owns repeated quantized tensor descriptions and
small shape/value helpers.

This reduces `scalar_llama.rs` from 572 lines to 365 lines. The new focused
tensor module is 214 lines.

## Verification

Commands:

```sh
cargo fmt --all -- --check
cargo test -p ferrite-fixtures -- --nocapture
cargo test -p ferrite-inference --test scalar_reference -- --nocapture
git diff --check
CARGO_BUILD_JOBS=2 cargo clippy -p ferrite-fixtures --all-targets -- -D warnings
```

Observed results:

- `cargo fmt --all -- --check`: passed.
- `cargo test -p ferrite-fixtures -- --nocapture`: crate compiled; 0 unit tests
  and 0 doc tests ran.
- `cargo test -p ferrite-inference --test scalar_reference -- --nocapture`: 22
  passed, 0 failed, 0 ignored, finished in 0.00s.
- `git diff --check`: passed.
- `CARGO_BUILD_JOBS=2 cargo clippy -p ferrite-fixtures --all-targets -- -D warnings`:
  passed in 7.95s.

## Result

Fixture construction remains behaviorally covered by downstream scalar GGUF
loading tests while quantized tensor definitions now live in a smaller focused
module.

## Limits

This is a structure-only refactor. It does not add new fixture variants, model
architectures, quantization formats, or parser behavior.
