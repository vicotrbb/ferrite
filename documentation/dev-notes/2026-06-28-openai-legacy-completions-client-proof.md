# OpenAI Legacy Completions Client Proof

Date: 2026-06-28

## Summary

Ferrite's `POST /v1/completions` endpoint is now covered by a live
`async-openai` client integration test.

The test starts a local Ferrite server, configures `async-openai` with the
Ferrite `/v1` base URL, calls `client.completions().create(...)`, and verifies
the parsed legacy text completion response.

## Implementation Notes

- Added `async_openai_client_creates_legacy_completion` to
  `crates/ferrite-server/tests/openai_client.rs`.
- Enabled the `async-openai` `completions` dev-dependency feature.
- No production code changes were needed; the existing `/v1/completions`
  response shape already matched the client type.

## Verification

Red test first:

```sh
cargo test -p ferrite-server --test openai_client async_openai_client_creates_legacy_completion -- --nocapture
```

Initial result before enabling the client feature:

- compile failed because `async_openai::types::completions` and
  `Client::completions()` were gated behind the `completions` feature.

Focused proof after enabling the feature:

```sh
cargo test -p ferrite-server --test openai_client async_openai_client_creates_legacy_completion -- --nocapture
```

Observed result:

- passed, proving the current `/v1/completions` response is parsed by
  `async-openai`.

Final verification:

```sh
cargo fmt --all
cargo test -p ferrite-server -- --nocapture
cargo clippy -p ferrite-server --all-targets -- -D warnings
cargo check --workspace
git diff --check
```

Observed result:

- `cargo test -p ferrite-server -- --nocapture`: 46 unit tests passed,
  6 `openai_client` integration tests passed, and 2 `openai_http` integration
  tests passed.
- `cargo clippy -p ferrite-server --all-targets -- -D warnings`: passed.
- `cargo check --workspace`: passed.
- `git diff --check`: passed.
