# OpenAI Real Tier 1 SmolLM2 1.7B Async Client Proof

Date: 2026-06-30

## Scope

This slice adds ignored `async-openai` integration coverage that drives the
local SmolLM2-1.7B Q4_K_M GGUF artifact through Ferrite's OpenAI-compatible
HTTP server.

The proof uses:

- Model artifact: `target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf`
- Model id: `smollm2-1.7b-q4_k_m`
- Override env var: `FERRITE_SMOLLM_1_7B_Q4_MODEL`
- Test file: `crates/ferrite-server/tests/openai_client_real_tier1_smollm_1_7b.rs`
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
cargo test -p ferrite-server --test openai_client_real_tier1_smollm_1_7b -- --ignored --nocapture
```

Observed result:

```text
running 4 tests
test async_openai_client_generates_with_smollm_1_7b_q4_model ... ok
test async_openai_client_chats_with_smollm_1_7b_q4_model has been running for over 60 seconds
test async_openai_client_streams_chat_with_smollm_1_7b_q4_model has been running for over 60 seconds
test async_openai_client_streams_with_smollm_1_7b_q4_model has been running for over 60 seconds
test async_openai_client_streams_with_smollm_1_7b_q4_model ... ok
test async_openai_client_chats_with_smollm_1_7b_q4_model ... ok
test async_openai_client_streams_chat_with_smollm_1_7b_q4_model ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 251.34s
```

## Result

Ferrite's current tree can serve the SmolLM2-1.7B Q4_K_M Tier 1 artifact
through a standard OpenAI-compatible Rust client configured with Ferrite's
local `/v1` base URL. The proof covers legacy completions and chat completions,
including SSE streaming for both endpoint families.

## Limits

This is real Tier 1 SmolLM2-1.7B Q4_K_M client proof only. It does not prove
throughput targets, memory budgets, multi-client concurrency, or full OpenAI
API parity.
