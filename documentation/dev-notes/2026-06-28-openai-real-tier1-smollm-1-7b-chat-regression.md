# 2026-06-28 OpenAI Real Tier 1 SmolLM2 1.7B Chat Regression

## Slice

This slice adds an opt-in real-model OpenAI-compatible HTTP chat regression for
SmolLM2-1.7B-Instruct Q4_K_M.

The existing SmolLM2 HTTP regressions covered six-prompt legacy completions and
streaming legacy completions. This test file proves the same real model can
also serve chat completions over the six-prompt reference set, plus streaming
chat completions for the canonical `hello world` prompt:

- `POST /v1/chat/completions`
- `POST /v1/chat/completions` with `stream: true`

## Test Added

```text
crates/ferrite-server/tests/openai_real_tier1_smollm_1_7b_chat.rs
```

The test:

- starts Ferrite's OpenAI-compatible server with
  `target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf`;
- sends six non-streaming chat completion requests;
- sends one streaming chat completion request;
- verifies HTTP `200` and OpenAI-shaped chat response objects;
- verifies prompt and completion usage for the non-streaming response;
- parses SSE `data:` JSON events for the streaming response;
- checks the generated delta content, terminal stop chunk, and `[DONE]`.

## Expected First Token

Ferrite's chat renderer turns the request into:

```text
user: hello world
assistant:
```

The expected result is:

| Prompt | Prompt tokens | First chat content |
| --- | ---: | --- |
| `hello world` | 9 | `1` |
| `The capital of France is` | 12 | `\n` |
| `Once upon a time` | 11 | `\n` |
| `Rust is a systems programming language` | 13 | `\n` |
| `Machine learning models can` | 11 | `1` |
| `The recipe calls for` | 11 | `1` |

## Validation

Compile-only default ignored test run:

```sh
cargo test -p ferrite-server --test openai_real_tier1_smollm_1_7b_chat
```

Result:

```text
test live_http_server_chats_with_smollm_1_7b_q4_reference_prompt ... ignored, requires local SmolLM2-1.7B Q4_K_M GGUF model artifact
test live_http_server_matches_smollm_1_7b_q4_chat_first_tokens_for_reference_prompts ... ignored, requires local SmolLM2-1.7B Q4_K_M GGUF model artifact
test result: ok. 0 passed; 0 failed; 2 ignored
```

Real local model run for the chat plus streaming chat smoke proof:

```sh
cargo test -p ferrite-server --test openai_real_tier1_smollm_1_7b_chat live_http_server_chats_with_smollm_1_7b_q4_reference_prompt -- --ignored --nocapture
```

Result:

```text
test live_http_server_chats_with_smollm_1_7b_q4_reference_prompt ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; finished in 131.06s
```

Real local model run for the six-prompt non-streaming chat proof:

```sh
cargo test -p ferrite-server --test openai_real_tier1_smollm_1_7b_chat live_http_server_matches_smollm_1_7b_q4_chat_first_tokens_for_reference_prompts -- --ignored --nocapture
```

Result:

```text
test live_http_server_matches_smollm_1_7b_q4_chat_first_tokens_for_reference_prompts ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 1 filtered out; finished in 477.08s
```

## Boundary

This proves the local OpenAI-compatible chat path can drive the real
SmolLM2-1.7B Q4_K_M model for six deterministic one-token prompt cases. It
also proves the streaming chat path for one deterministic one-token prompt
case. It does not prove six-prompt SmolLM2 streaming chat behavior, SmolLM2
chat throughput, broad concurrent serving, or full Tier 1 completion.
