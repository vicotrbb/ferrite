# OpenAI Model Retrieve

Date: 2026-06-28

## Summary

Ferrite's OpenAI-compatible server now supports `GET /v1/models/{model}` for
retrieving the loaded local model's metadata.

The endpoint returns the same `ModelObject` shape used by `GET /v1/models`.
Unknown model ids return an OpenAI-shaped `invalid_request_error` with status
`404` and code `model_not_found`.

## Implementation Notes

- Added a focused catalog test module at
  `crates/ferrite-server/src/openai/catalog_tests.rs` instead of growing the
  generation route-test file.
- Added a narrow `OpenAiHttpError::model_not_found` constructor so model
  retrieval can return a precise compatibility error without changing existing
  generation error behavior.
- Kept model metadata construction in `openai::schema::catalog`.

## Compatibility Reference

OpenAI's Models API documents `GET /models/{model}` as the retrieve-model
operation and returns a model object containing `id`, `object`, `created`, and
`owned_by`.

Source retrieved 2026-06-28:
<https://platform.openai.com/docs/api-reference/models/retrieve>

## Verification

Red tests first:

```sh
cargo test -p ferrite-server openai::catalog_tests -- --nocapture
```

Initial result before implementation:

- `model_retrieve_endpoint_returns_loaded_model` returned router fallback
  `404` instead of `200`.
- `model_retrieve_endpoint_rejects_unknown_model` returned an empty fallback
  body instead of an OpenAI-shaped error object.

Final verification:

```sh
cargo test -p ferrite-server -- --nocapture
cargo clippy -p ferrite-server --all-targets -- -D warnings
cargo check --workspace
git diff --check
```

Observed result:

- `cargo test -p ferrite-server -- --nocapture`: 19 passed.
- `cargo clippy -p ferrite-server --all-targets -- -D warnings`: passed.
- `cargo check --workspace`: passed.
- `git diff --check`: passed.
