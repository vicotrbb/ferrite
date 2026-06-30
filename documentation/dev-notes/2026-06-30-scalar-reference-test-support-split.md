# Scalar Reference Test Support Split

Date: 2026-06-30

## Scope

This slice splits repeated scalar-reference integration-test setup out of
`crates/ferrite-inference/tests/scalar_reference.rs` without changing the
tested behaviors.

The refactor adds:

- `crates/ferrite-inference/tests/support/assertions.rs` for shared close-float
  assertions.
- `crates/ferrite-inference/tests/support/fixtures.rs` for the Qwen2 fixture
  mutation helper.
- `crates/ferrite-inference/tests/support/models.rs` for small hand-built
  scalar Llama test models.
- `crates/ferrite-inference/tests/support/mod.rs` as the integration-test
  support module boundary.

This reduces `scalar_reference.rs` from 717 lines to 346 lines. The new support
modules remain focused, with the largest support file at 194 lines.

## Verification

Commands:

```sh
cargo test -p ferrite-inference --test scalar_reference -- --nocapture
cargo fmt --all -- --check
git diff --check
CARGO_BUILD_JOBS=2 cargo clippy -p ferrite-inference --all-targets -- -D warnings
```

Observed results:

- `cargo test -p ferrite-inference --test scalar_reference -- --nocapture`: 22
  passed, 0 failed, 0 ignored, finished in 0.01s.
- `cargo fmt --all -- --check`: passed.
- `git diff --check`: passed.
- `CARGO_BUILD_JOBS=2 cargo clippy -p ferrite-inference --all-targets -- -D warnings`:
  passed in 7.03s.

## Result

The scalar reference coverage remains intact while repeated mini-model setup
now lives behind small test support builders.

## Limits

This is a structure-only refactor. It does not change inference behavior, add
new model tiers, or add new reference comparisons.
