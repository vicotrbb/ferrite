# OpenAI JSON Request Errors

Date: 2026-06-28

## Summary

Ferrite's OpenAI-compatible POST routes now map JSON extraction failures to
OpenAI-shaped `invalid_request_error` responses. This covers malformed JSON and
requests missing the `content-type: application/json` header.

## Implementation Notes

- Added `crates/ferrite-server/src/openai/json.rs`.
- `OpenAiJson<T>` wraps axum's `Json<T>` extractor and maps
  `JsonRejection` into Ferrite's existing `OpenAiHttpError::invalid_request`.
- Only OpenAI POST routes use this extractor; health and model-list routes stay
  unchanged.

## Verification

Red tests first:

```sh
cargo test -p ferrite-server -- openai::routes_tests::completions_endpoint_returns_openai_error_for_malformed_json openai::routes_tests::completions_endpoint_returns_openai_error_for_missing_json_content_type -- --nocapture
```

Initial result:

- malformed JSON test failed because the body was not an OpenAI error object.
- missing JSON content type test failed with `415` instead of `400`.

Final verification:

```sh
cargo test -p ferrite-server -- --nocapture
cargo clippy -p ferrite-server --all-targets -- -D warnings
cargo check --workspace
git diff --check
```

Observed result:

- `cargo test -p ferrite-server -- --nocapture`: 13 passed.
- `cargo clippy -p ferrite-server --all-targets -- -D warnings`: passed.
- `cargo check --workspace`: passed.
- `git diff --check`: passed.
