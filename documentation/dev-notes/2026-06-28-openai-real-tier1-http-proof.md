# OpenAI Real Tier 1 HTTP Proof

Date: 2026-06-28

## Summary

Ferrite now has an opt-in live HTTP integration test that runs the
OpenAI-compatible legacy completions endpoint against a real Tier 1 GGUF model.

The proof starts a live Axum server with
`target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`, sends a raw HTTP/1.1 request
to `POST /v1/completions`, and verifies the deterministic first generated
token for `hello world`.

## Expected Output Probe

CLI probe:

```sh
target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --prompt 'hello world' --generate-tokens 1
```

Observed output:

```text
prompt_token_ids=14990,1879
generated_token_ids=198
generated_text=
```

The generated token is a newline, so the HTTP test asserts `"\n"`.

## Implementation Notes

- Added `crates/ferrite-server/tests/openai_real_tier1_http.rs`.
- The test is ignored by default because it requires a local Tier 1 model
  artifact and loads a 379 MB GGUF file.
- The default model path is
  `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`.
- The model path can be overridden with `FERRITE_REAL_TIER1_MODEL`.
- No production server code changed.

## Verification

Explicit Tier 1 HTTP proof:

```sh
cargo test -p ferrite-server --test openai_real_tier1_http -- --ignored --nocapture
```

Observed result:

- 1 ignored Tier 1 real-model HTTP test passed when explicitly enabled.
- Rust test harness time for the target: about 14.19s.

The test verifies:

- HTTP status `200 OK`.
- response object `text_completion`.
- response model `qwen2.5-0.5b`.
- generated text `"\n"` for prompt `hello world` with `max_tokens: 1`.
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
  tests passed, 4 real Tier 0 HTTP tests were ignored by default, and 1 real
  Tier 1 HTTP test was ignored by default.
- `cargo clippy -p ferrite-server --all-targets -- -D warnings`: passed.
- `git diff --check`: passed.

## Boundary

This proves real Tier 1 Qwen2.5-0.5B Q4_K_M execution through the
non-streaming OpenAI-compatible legacy completions HTTP path. It does not prove
Tier 1 HTTP streaming, Tier 1 chat, Tier 1 throughput through the server, or
broader Tier 1 model coverage.
