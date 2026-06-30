# OpenAI real Tier 1 Qwen 0.5B async client proof

## Context

Ferrite's OpenAI-compatible HTTP server needs progressive proof beyond fixture
engines and Tier 0 models. The local workspace has the Tier 1 Qwen2.5 0.5B
Q4_K_M GGUF artifact available at:

- `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`

## Slice

Run the ignored `async-openai` Tier 1 integration test file sequentially. This
covers:

- `POST /v1/completions`
- streaming `POST /v1/completions`
- `POST /v1/chat/completions`
- streaming `POST /v1/chat/completions`

All requests use the standard `async-openai` client configured with Ferrite's
local `/v1` base URL.

## Validation

Executed:

- `cargo test -p ferrite-server --test openai_client_real_tier1 -- --ignored --nocapture --test-threads=1`

Result:

- 4 passed, 0 failed, finished in 59.44s.

This proves the local OpenAI-compatible server can generate through the real
Tier 1 Qwen2.5 0.5B Q4_K_M GGUF model on the tested client paths.
