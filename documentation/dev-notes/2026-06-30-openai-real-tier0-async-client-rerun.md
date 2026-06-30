# OpenAI Real Tier 0 Async Client Rerun

Date: 2026-06-30

## Context

This slice reran the ignored real Tier 0 OpenAI-compatible client proof after
the current server hardening work. The goal was to verify the current tree still
serves a real Tier 0 GGUF model through a standard OpenAI client configured with
Ferrite's local `/v1` base URL.

## Model Artifact

- Model: `SmolLM2-135M-Instruct-Q4_K_M.gguf`
- Path: `target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf`
- Commit under test: `2464b8a`

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

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 20.99s
```

## Coverage

This proves the current tree can serve the local SmolLM2-135M Q4_K_M artifact
through the `async-openai` client for:

- `POST /v1/completions`;
- streamed `POST /v1/completions`;
- `POST /v1/chat/completions`;
- streamed `POST /v1/chat/completions`.

This does not prove Tier 1 or larger models, broad prompt quality, long-context
behavior, hosted OpenAI API parity, or throughput.
