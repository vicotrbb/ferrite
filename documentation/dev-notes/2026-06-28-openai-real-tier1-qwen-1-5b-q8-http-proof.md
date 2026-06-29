# 2026-06-28 OpenAI Real Tier 1 Qwen2.5 1.5B Q8_0 HTTP Proof

## Scope

This note records explicit ignored integration tests for serving
Qwen2.5-1.5B-Instruct Q8_0 through the OpenAI-compatible legacy completions
endpoint, both non-streaming and SSE streaming, and through the chat
completions endpoint.

This proves deterministic real Tier 1 Q8_0 completion requests through the HTTP
server. It does not prove streaming chat, concurrent serving, queue fairness,
or HTTP throughput for the 1.5B Q8_0 model.

## Test Added

- `crates/ferrite-server/tests/openai_real_tier1_qwen_1_5b_http.rs`
- Tests:
  - `live_http_server_generates_with_qwen_1_5b_q8_model`
  - `live_http_server_streams_with_qwen_1_5b_q8_model`
  - `live_http_server_chats_with_qwen_1_5b_q8_model`
- Default model path:
  `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
- Override env var: `FERRITE_QWEN_1_5B_Q8_MODEL`
- HTTP endpoint: `POST /v1/completions`
- Chat endpoint: `POST /v1/chat/completions`
- Model id: `qwen2.5-1.5b-q8_0`
- Prompt: `hello world`
- Max tokens: 1

## TDD Evidence

Red command:

```sh
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_http live_http_server_generates_with_qwen_1_5b_q8_model -- --ignored --nocapture
```

Expected red failure:

```text
error[E0425]: cannot find function `qwen_1_5b_q8_model_path` in this scope
```

After adding the model path helper and default artifact path, the focused test
passed:

```text
test live_http_server_generates_with_qwen_1_5b_q8_model ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 28.91s
```

The response assertions verify:

- HTTP `200 OK`;
- OpenAI `text_completion` object shape;
- response model id `qwen2.5-1.5b-q8_0`;
- completion text `\n`;
- prompt token count 2;
- completion token count 1; and
- total token count 3.

The streaming proof was added with a second red/green check. Red command:

```sh
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_http live_http_server_streams_with_qwen_1_5b_q8_model -- --ignored --nocapture
```

Expected red failure:

```text
error[E0425]: cannot find function `assert_qwen_1_5b_q8_stream_response` in this scope
```

After adding the local SSE assertion helper, the focused streaming test passed:

```text
test live_http_server_streams_with_qwen_1_5b_q8_model ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 28.98s
```

The streaming response assertions verify:

- HTTP `200 OK`;
- `content-type: text/event-stream`;
- OpenAI `text_completion` stream object shape;
- response model id `qwen2.5-1.5b-q8_0`;
- streamed token text `\n`; and
- terminal `data: [DONE]`.

The chat proof was added with a third red/green check. Red command:

```sh
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_http live_http_server_chats_with_qwen_1_5b_q8_model -- --ignored --nocapture
```

Expected red failure:

```text
error[E0425]: cannot find function `assert_qwen_1_5b_q8_chat_response` in this scope
```

After adding the local chat response assertion helper, the focused chat test
passed:

```text
test live_http_server_chats_with_qwen_1_5b_q8_model ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 81.08s
```

The chat response assertions verify:

- HTTP `200 OK`;
- OpenAI `chat.completion` object shape;
- response model id `qwen2.5-1.5b-q8_0`;
- assistant message content `你好`;
- prompt token count 8;
- completion token count 1; and
- total token count 9.

## Verification

The explicit ignored real-model test file passed again:

```sh
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_http -- --ignored --nocapture
```

```text
test live_http_server_streams_with_qwen_1_5b_q8_model ... ok
test live_http_server_generates_with_qwen_1_5b_q8_model ... ok
test live_http_server_chats_with_qwen_1_5b_q8_model ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 98.50s
```

Normal server verification also passed:

```sh
cargo fmt --all -- --check
cargo clippy -p ferrite-server --all-targets -- -D warnings
git diff --check
```

`cargo test -p ferrite-server -- --nocapture` passed during the non-streaming
slice: 50 server unit tests, 7 `async-openai` client integration tests, and
6 fixture live HTTP integration tests. The real model tests remained ignored
unless explicitly selected.

## Result

Ferrite now has explicit OpenAI-compatible HTTP proofs for local
Qwen2.5-1.5B Q8_0 legacy completion serving, both non-streaming and SSE
streaming, plus non-streaming chat completion serving. This follows the CLI
throughput slice that measured the same model above 10 tok/s locally, but HTTP
throughput for this model remains unmeasured.
