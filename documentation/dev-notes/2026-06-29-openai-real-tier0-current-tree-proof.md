# OpenAI Real Tier 0 Current-Tree HTTP Proof

Date: 2026-06-29

## Scope

This note records a fresh current-tree rerun of Ferrite's OpenAI-compatible HTTP
integration test against a real Tier 0 GGUF model.

The proof uses:

- Model artifact: `target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf`
- Model id: `smollm2-135m`
- Test file: `crates/ferrite-server/tests/openai_real_model_http.rs`
- Endpoints:
  - `POST /v1/completions`
  - `POST /v1/completions` with `stream: true`
  - `POST /v1/chat/completions`
  - `POST /v1/chat/completions` with `stream: true`

## Verification

Command:

```sh
cargo test -p ferrite-server --test openai_real_model_http -- --ignored --nocapture
```

Observed result:

```text
running 4 tests
test live_http_server_generates_with_real_tier0_model ... ok
test live_http_server_streams_with_real_tier0_model ... ok
test live_http_server_chats_with_real_tier0_model ... ok
test live_http_server_streams_chat_with_real_tier0_model ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 22.04s
```

## Result

The current tree successfully drives a real SmolLM2-135M Q4_K_M model through
the OpenAI-compatible local HTTP server for legacy completions and chat
completions, including SSE streaming for both endpoints.

## Limits

This is Tier 0 HTTP proof only. It does not prove Tier 1 or larger model
serving on the current tree, OpenAI SDK client compatibility, throughput,
memory behavior, concurrent serving, or broader OpenAI API parity.
