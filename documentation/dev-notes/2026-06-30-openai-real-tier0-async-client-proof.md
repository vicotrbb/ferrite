# OpenAI real Tier 0 async client proof

## Context

Ferrite's OpenAI-compatible HTTP server must work with common OpenAI clients
against real model execution, not only fixture engines. The local workspace has
the Tier 0 SmolLM2 135M Q4_K_M GGUF artifact available at:

- `target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf`

## Slice

Run the ignored `async-openai` Tier 0 integration test file sequentially. This
covers:

- `POST /v1/completions`
- streaming `POST /v1/completions`
- `POST /v1/chat/completions`
- streaming `POST /v1/chat/completions`

All requests use the standard `async-openai` client configured with Ferrite's
local `/v1` base URL.

## Validation

Executed:

- `cargo test -p ferrite-server --test openai_client_real_tier0 -- --ignored --nocapture --test-threads=1`

Result:

- 4 passed, 0 failed, finished in 48.04s.

This proves the local OpenAI-compatible server can generate through a real Tier
0 GGUF model on the tested client paths.
