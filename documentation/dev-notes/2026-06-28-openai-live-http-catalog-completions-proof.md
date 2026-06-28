# OpenAI Live HTTP Catalog and Completions Proof

Date: 2026-06-28

## Summary

Ferrite's live HTTP integration coverage now exercises curl-style raw HTTP
requests for more of the first OpenAI-compatible server surface.

The existing raw TCP test covered `POST /v1/chat/completions`. This slice adds
direct live-server coverage for:

- `GET /v1/models`
- `POST /v1/completions`

## Implementation Notes

- Added `live_http_server_accepts_openai_style_model_list` to
  `crates/ferrite-server/tests/openai_http.rs`.
- Added `live_http_server_accepts_openai_style_legacy_completion` to
  `crates/ferrite-server/tests/openai_http.rs`.
- Extracted a small `response_json` helper in the same test file to avoid
  repeating HTTP body splitting logic.
- No production code changed. The proof uses the deterministic fixture model,
  not a real GGUF model.

## Verification

Focused proof:

```sh
cargo test -p ferrite-server --test openai_http -- --nocapture
```

Observed result:

- 4 live HTTP tests passed.

Server verification before the test commit:

```sh
cargo fmt --all
cargo test -p ferrite-server -- --nocapture
cargo clippy -p ferrite-server --all-targets -- -D warnings
git diff --check
```

Observed result:

- `cargo test -p ferrite-server -- --nocapture`: 48 unit tests passed,
  7 `openai_client` integration tests passed, and 4 `openai_http` integration
  tests passed.
- `cargo clippy -p ferrite-server --all-targets -- -D warnings`: passed.
- `git diff --check`: passed.

## Remaining HTTP Proof Gaps

- Raw live HTTP streaming coverage is still not present; streaming is covered
  by in-process route tests and `async-openai` client integration tests.
- The live HTTP tests still use a fixture model for speed and determinism.
  Real-model server proof remains future work after selecting a bounded model
  and runtime protocol.
