# OpenAI Real Tier 0 Async Client Proof

Date: 2026-06-29

## Scope

This slice adds ignored `async-openai` integration coverage that drives a real
Tier 0 GGUF model through Ferrite's OpenAI-compatible HTTP server.

The proof uses:

- Model artifact: `target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf`
- Model id: `smollm2-135m`
- Test file: `crates/ferrite-server/tests/openai_client_real_tier0.rs`
- Client base URL: `http://<server>/v1`
- Client crate: `async-openai`

Covered client paths:

- `client.completions().create(...)`
- `client.completions().create_stream(...)`
- `client.chat().create(...)`
- `client.chat().create_stream(...)`

## Verification

Command:

```sh
cargo test -p ferrite-server --test openai_client_real_tier0 -- --ignored --nocapture
```

Observed result:

```text
running 4 tests
test async_openai_client_streams_with_real_tier0_model ... ok
test async_openai_client_generates_with_real_tier0_model ... ok
test async_openai_client_chats_with_real_tier0_model ... ok
test async_openai_client_streams_chat_with_real_tier0_model ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 30.65s
```

## Result

Ferrite's current tree can serve a real SmolLM2-135M Q4_K_M model through a
standard OpenAI-compatible Rust client configured with Ferrite's local `/v1`
base URL. The proof covers legacy completions and chat completions, including
SSE streaming for both endpoint families.

## Limits

This is real Tier 0 client proof only. It does not prove Tier 1 or larger
models through `async-openai`, full OpenAI API parity, production concurrency,
throughput, or memory behavior.
