# OpenAI Legacy Completions Stream Client Proof

Date: 2026-06-28

## Summary

Ferrite's `POST /v1/completions` streaming mode is now covered by a live
`async-openai` client integration test.

The test starts a local Ferrite server, configures `async-openai` with the
Ferrite `/v1` base URL, calls `client.completions().create_stream(...)`, reads
parsed stream chunks, reconstructs the generated text, and verifies the final
usage chunk from `stream_options.include_usage`.

## Implementation Notes

- Added `async_openai_client_streams_legacy_completion` to
  `crates/ferrite-server/tests/openai_client.rs`.
- The proof uses `ChatCompletionStreamOptions { include_usage: Some(true) }`
  on the legacy completions request because `async-openai` uses the same stream
  options type for completions and chat completions.
- No production code changes were needed; the existing `/v1/completions` SSE
  response shape already matched the client stream parser.
- This proof uses the deterministic in-repo fixture model output, not a real
  GGUF model. Real-model correctness remains tracked by the Tier 0 and Tier 1
  gate documents.

## Verification

Focused proof:

```sh
cargo test -p ferrite-server --test openai_client async_openai_client_streams_legacy_completion -- --nocapture
```

Observed result:

- passed, proving the existing streaming legacy completions endpoint is
  consumable by the standard `async-openai` stream client.

Final verification run before committing the test slice:

```sh
cargo fmt --all
cargo test -p ferrite-server -- --nocapture
cargo clippy -p ferrite-server --all-targets -- -D warnings
cargo check --workspace
git diff --check
```

Observed result:

- `cargo test -p ferrite-server -- --nocapture`: 46 unit tests passed,
  7 `openai_client` integration tests passed, and 2 `openai_http` integration
  tests passed.
- `cargo clippy -p ferrite-server --all-targets -- -D warnings`: passed.
- `cargo check --workspace`: passed.
- `git diff --check`: passed.
