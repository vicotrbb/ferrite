# OpenAI Real Tier 1 Qwen2.5 1.5B Q6_K Async Client Proof

Date: 2026-06-30

## Scope

This slice adds ignored `async-openai` integration coverage that drives the
local Qwen2.5-1.5B Q6_K GGUF artifact through Ferrite's OpenAI-compatible HTTP
server.

The proof uses:

- Model artifact: `target/models/qwen2.5-1.5b-instruct-q6_k.gguf`
- Model id: `qwen2.5-1.5b-q6_k`
- Override env var: `FERRITE_QWEN_1_5B_Q6_MODEL`
- Test file: `crates/ferrite-server/tests/openai_client_real_tier1_qwen_1_5b_q6.rs`
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
cargo test -p ferrite-server --test openai_client_real_tier1_qwen_1_5b_q6 -- --ignored --nocapture
```

Observed result:

```text
running 4 tests
test async_openai_client_chats_with_qwen_1_5b_q6_model has been running for over 60 seconds
test async_openai_client_generates_with_qwen_1_5b_q6_model has been running for over 60 seconds
test async_openai_client_streams_chat_with_qwen_1_5b_q6_model has been running for over 60 seconds
test async_openai_client_streams_with_qwen_1_5b_q6_model has been running for over 60 seconds
test async_openai_client_streams_chat_with_qwen_1_5b_q6_model ... ok
test async_openai_client_chats_with_qwen_1_5b_q6_model ... ok
test async_openai_client_streams_with_qwen_1_5b_q6_model ... ok
test async_openai_client_generates_with_qwen_1_5b_q6_model ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 285.94s
```

## Result

Ferrite's current tree can serve the Qwen2.5-1.5B Q6_K Tier 1 artifact through
a standard OpenAI-compatible Rust client configured with Ferrite's local `/v1`
base URL. The proof covers legacy completions and chat completions, including
SSE streaming for both endpoint families.

## Limits

This is real Tier 1 Qwen2.5-1.5B Q6_K client proof only. It does not prove
throughput targets, memory budgets, multi-client concurrency, or full OpenAI
API parity.
