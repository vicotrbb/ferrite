# OpenAI Real Tier 0 HTTP Proof

Date: 2026-06-28

## Summary

Ferrite now has an opt-in live HTTP integration test that runs the
OpenAI-compatible legacy completions endpoint against a real Tier 0 GGUF model.

The proof starts a live Axum server with
`target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf`, sends a raw HTTP/1.1 request
to `POST /v1/completions`, and verifies the deterministic first generated
token for `hello world`.

## Implementation Notes

- Added `crates/ferrite-server/tests/openai_real_model_http.rs`.
- The test is ignored by default because it requires a local model artifact and
  takes several seconds to load the GGUF model.
- The default model path is
  `target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf`.
- The model path can be overridden with `FERRITE_REAL_MODEL`.
- Extended `crates/ferrite-server/tests/support/mod.rs` so live test servers
  can be started from an existing GGUF path without deleting the artifact on
  drop.

## Verification

First explicit run exposed a test-path issue:

```sh
cargo test -p ferrite-server --test openai_real_model_http -- --ignored --nocapture
```

Observed result:

- failed with `No such file or directory` because the default model path was
  resolved from the integration-test process directory instead of the repo
  root.

After resolving the default path from `CARGO_MANIFEST_DIR`:

```sh
cargo test -p ferrite-server --test openai_real_model_http -- --ignored --nocapture
```

Observed result:

- 1 ignored real-model test passed when explicitly enabled.
- Wall-clock test body time reported by the Rust test harness: about 7.89s.

The test verifies:

- HTTP status `200 OK`.
- response object `text_completion`.
- response model `smollm2-135m`.
- generated text `"."` for prompt `hello world` with `max_tokens: 1`.
- usage counts: 2 prompt tokens, 1 completion token, 3 total tokens.

Default server verification before the test commit:

```sh
cargo fmt --all
cargo test -p ferrite-server -- --nocapture
cargo clippy -p ferrite-server --all-targets -- -D warnings
git diff --check
```

Observed result:

- `cargo test -p ferrite-server -- --nocapture`: 48 unit tests passed,
  7 `openai_client` integration tests passed, 6 `openai_http` integration
  tests passed, and the real-model HTTP test was ignored by default.
- `cargo clippy -p ferrite-server --all-targets -- -D warnings`: passed.
- `git diff --check`: passed.

## Boundary

This proves the OpenAI-compatible HTTP path can drive a real Tier 0 GGUF model
for a deterministic one-token legacy completion. It does not yet prove real
GGUF chat completions, real-model streaming over HTTP, or Tier 1+ server
behavior.
