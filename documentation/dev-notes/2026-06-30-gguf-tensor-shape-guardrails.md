# GGUF Tensor Shape Guardrails

Date: 2026-06-30

## Scope

This slice hardens GGUF tensor-info parsing against invalid tensor shapes:

- tensors with zero dimensions are rejected;
- tensors with any dimension equal to zero are rejected.

The validation lives in `crates/ferrite-model/src/gguf/reader.rs` while reading
tensor info, before byte ranges or downstream model loaders can observe the
malformed tensor.

## Red-Green Evidence

Red commands before implementation:

```sh
cargo test -p ferrite-model --test gguf_reader rejects_tensors_with_no_dimensions -- --nocapture
cargo test -p ferrite-model --test gguf_reader rejects_tensors_with_zero_dimensions -- --nocapture
```

Observed red results:

- `rejects_tensors_with_no_dimensions`: failed because an empty tensor shape was
  accepted.
- `rejects_tensors_with_zero_dimensions`: failed because a zero dimension was
  accepted.

Green commands after implementation:

```sh
cargo test -p ferrite-model --test gguf_reader rejects_tensors_with_no_dimensions -- --nocapture
cargo test -p ferrite-model --test gguf_reader rejects_tensors_with_zero_dimensions -- --nocapture
cargo test -p ferrite-model --test gguf_reader -- --nocapture
cargo fmt --all -- --check
git diff --check
CARGO_BUILD_JOBS=2 cargo clippy -p ferrite-model --all-targets -- -D warnings
```

Observed green results:

- `rejects_tensors_with_no_dimensions`: 1 passed, 0 failed.
- `rejects_tensors_with_zero_dimensions`: 1 passed, 0 failed.
- `cargo test -p ferrite-model --test gguf_reader -- --nocapture`: 10 passed,
  0 failed, 0 ignored, finished in 0.00s.
- `cargo fmt --all -- --check`: passed after applying rustfmt.
- `git diff --check`: passed.
- `CARGO_BUILD_JOBS=2 cargo clippy -p ferrite-model --all-targets -- -D warnings`:
  passed in 14.19s.

## Result

Malformed GGUF tensor metadata with empty or zero-sized shapes now fails
deterministically during parsing.

## Limits

This is a focused parser guardrail. It does not add property-based fuzzing,
upper bounds on dimension count, or broader tensor-shape compatibility rules.
