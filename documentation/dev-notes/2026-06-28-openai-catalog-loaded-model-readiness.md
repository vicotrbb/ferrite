# OpenAI Catalog Loaded-Model Readiness

Date: 2026-06-28

## Summary

Ferrite's OpenAI-compatible model catalog no longer advertises a configured
model id when no inference engine is loaded.

This keeps `/v1/models` and `/v1/models/{model}` aligned with the actual local
serving state: a configured id alone is not enough to claim a model is
available to clients.

## Implementation Notes

- Added `ServerState::has_loaded_model()` as the narrow readiness check for
  catalog routes.
- Changed `GET /v1/models` to return an empty OpenAI-shaped list when the
  server has no loaded engine.
- Changed `GET /v1/models/{model}` to return the existing OpenAI-shaped
  `model_not_found` error when the requested model id matches the configured
  id but no engine is loaded.
- Updated loaded-model catalog tests to use an actual fixture-backed
  `InferenceEngine` instead of `ServerState::new(...)`.
- This slice tightens HTTP catalog correctness only. It does not add new
  real-model inference evidence.

## Verification

Red tests:

```sh
cargo test -p ferrite-server openai::catalog_tests::models_endpoint_returns_empty_list_when_no_model_is_loaded -- --nocapture
cargo test -p ferrite-server openai::catalog_tests::model_retrieve_endpoint_rejects_configured_id_when_no_model_is_loaded -- --nocapture
```

Observed result before the implementation:

- `/v1/models` returned a listed model even with no loaded engine.
- `/v1/models/{model}` returned `200` for the configured id even with no loaded
  engine.

Focused proof after implementation:

```sh
cargo test -p ferrite-server openai::catalog_tests -- --nocapture
```

Observed result:

- 4 catalog tests passed.

Server verification before the code commit:

```sh
cargo fmt --all
cargo test -p ferrite-server -- --nocapture
cargo clippy -p ferrite-server --all-targets -- -D warnings
git diff --check
```

Observed result:

- `cargo test -p ferrite-server -- --nocapture`: 48 unit tests passed,
  7 `openai_client` integration tests passed, and 2 `openai_http` integration
  tests passed.
- `cargo clippy -p ferrite-server --all-targets -- -D warnings`: passed.
- `git diff --check`: passed.
