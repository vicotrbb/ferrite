# OpenAI Real Tier 1 Async Client Proof

Date: 2026-06-30

## Scope

This slice adds ignored `async-openai` integration coverage that drives a real
Tier 1 GGUF model through Ferrite's OpenAI-compatible HTTP server.

The proof uses:

- Model artifact: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- Model id: `qwen2.5-0.5b`
- Test file: `crates/ferrite-server/tests/openai_client_real_tier1.rs`
- Client base URL: `http://<server>/v1`
- Client crate: `async-openai`

Covered client paths:

- `client.completions().create(...)`
- `client.completions().create_stream(...)`
- `client.chat().create(...)`
- `client.chat().create_stream(...)`

The test file serializes its real-model cases with a file-local Tokio mutex so
the default Rust test harness does not run several CPU-heavy model-serving
client tests at the same time.

## Verification

Command:

```sh
cargo test -p ferrite-server --test openai_client_real_tier1 -- --ignored --nocapture
```

Observed result:

```text
running 4 tests
test async_openai_client_generates_with_real_tier1_model ... ok
test async_openai_client_chats_with_real_tier1_model ... ok
test async_openai_client_streams_chat_with_real_tier1_model ... ok
test async_openai_client_streams_with_real_tier1_model has been running for over 60 seconds
test async_openai_client_streams_with_real_tier1_model ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 64.72s
```

## Result

Ferrite's current tree can serve a real Qwen2.5-0.5B Q4_K_M model through a
standard OpenAI-compatible Rust client configured with Ferrite's local `/v1`
base URL. The proof covers legacy completions and chat completions, including
SSE streaming for both endpoint families.

## Limits

This is real Tier 1 Qwen2.5-0.5B client proof only. It does not prove larger
Tier 1 artifacts through `async-openai`, full OpenAI API parity, production
concurrency, throughput, or memory behavior.
