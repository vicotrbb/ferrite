# OpenAI real Tier 1 Qwen 1.5B default obfuscation chat rerun

## Context

The Qwen2.5-1.5B Q8_0 `async-openai` 32-token chat path is the current larger
Tier 1 client proof for longer byte-level BPE streaming decode. After enabling
default stream obfuscation in the HTTP routes and reusable stream contexts, the
path needed a current-tree release rerun.

The local workspace has the required Tier 1 Qwen2.5 1.5B Q8_0 GGUF artifact:

- `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`

## Slice

Rerun the ignored release `async-openai` 32-token chat proof for Qwen2.5-1.5B
Q8_0. This single test covers:

- non-streaming `POST /v1/chat/completions`
- streaming `POST /v1/chat/completions`
- 32 completion tokens on both paths
- `stream_options.include_obfuscation: null` on the streaming path

## Validation

Executed:

- `cargo test --release -p ferrite-server --test openai_client_real_tier1_qwen_1_5b_q8_long_chat async_openai_client_chats_32_tokens_with_qwen_1_5b_q8_model -- --ignored --test-threads=1 --nocapture`

Result:

- 1 passed, 0 failed, finished in 10.52s.

This proves that the current release tree still serves Qwen2.5-1.5B Q8_0
through the tested longer `async-openai` chat create and stream path after
default stream-obfuscation compatibility changes. It does not prove every Tier
1 quantization, the legacy completion 32-token client path, high concurrency,
long-running leak freedom, or full OpenAI API parity.
