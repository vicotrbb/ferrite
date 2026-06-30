# GGUF Duplicate Key Guardrails

Date: 2026-06-30

## Scope

This slice hardens GGUF parsing against ambiguous duplicate identifiers:

- duplicate metadata keys are rejected instead of silently overwriting the
  earlier value in the metadata map;
- duplicate tensor names are rejected instead of allowing ambiguous
  `GgufFile::tensor(name)` lookup.

Both checks happen during `parse_gguf`, before tensor data ranges are exposed to
downstream model loading.

## Red-Green Evidence

Red commands before implementation:

```sh
cargo test -p ferrite-model --test gguf_reader rejects_duplicate_metadata_keys -- --nocapture
cargo test -p ferrite-model --test gguf_reader rejects_duplicate_tensor_names -- --nocapture
```

Observed red results:

- `rejects_duplicate_metadata_keys`: failed because duplicate metadata was
  accepted.
- `rejects_duplicate_tensor_names`: failed because duplicate tensor names were
  accepted.

Green commands after implementation:

```sh
cargo test -p ferrite-model --test gguf_reader rejects_duplicate_metadata_keys -- --nocapture
cargo test -p ferrite-model --test gguf_reader rejects_duplicate_tensor_names -- --nocapture
cargo test -p ferrite-model --test gguf_reader -- --nocapture
cargo fmt --all -- --check
git diff --check
CARGO_BUILD_JOBS=2 cargo clippy -p ferrite-model --all-targets -- -D warnings
```

Observed green results:

- `rejects_duplicate_metadata_keys`: 1 passed, 0 failed.
- `rejects_duplicate_tensor_names`: 1 passed, 0 failed.
- `cargo test -p ferrite-model --test gguf_reader -- --nocapture`: 8 passed,
  0 failed, 0 ignored, finished in 0.00s.
- `cargo fmt --all -- --check`: passed.
- `git diff --check`: passed.
- `CARGO_BUILD_JOBS=2 cargo clippy -p ferrite-model --all-targets -- -D warnings`:
  passed in 23.69s.

## Result

Malformed GGUF inputs with duplicate metadata keys or tensor names now fail
deterministically with explicit parser errors.

## Limits

This does not add broader property-based fuzzing or validation for every GGUF
metadata invariant. It is a focused binary parser guardrail.
