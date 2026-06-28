# 2026-06-28 OpenAI Real Tier 1 Qwen2.5 1.5B Q8_0 HTTP Proof

## Scope

This slice adds an explicit ignored integration test for serving
Qwen2.5-1.5B-Instruct Q8_0 through the OpenAI-compatible legacy completions
endpoint.

This proves one deterministic real Tier 1 Q8_0 model request through the HTTP
server. It does not prove streaming, chat, concurrent serving, queue fairness,
or HTTP throughput for the 1.5B Q8_0 model.

## Test Added

- `crates/ferrite-server/tests/openai_real_tier1_qwen_1_5b_http.rs`
- Test: `live_http_server_generates_with_qwen_1_5b_q8_model`
- Default model path:
  `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
- Override env var: `FERRITE_QWEN_1_5B_Q8_MODEL`
- HTTP endpoint: `POST /v1/completions`
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

## Verification

The explicit ignored real-model test passed again:

```sh
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_http -- --ignored --nocapture
```

```text
test live_http_server_generates_with_qwen_1_5b_q8_model ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 28.82s
```

Normal server verification also passed:

```sh
cargo fmt --all -- --check
cargo test -p ferrite-server -- --nocapture
cargo clippy -p ferrite-server --all-targets -- -D warnings
```

`cargo test -p ferrite-server -- --nocapture` passed 50 server unit tests,
7 `async-openai` client integration tests, and 6 fixture live HTTP integration
tests. The real model tests remained ignored unless explicitly selected.

## Result

Ferrite now has an explicit OpenAI-compatible HTTP proof for local
Qwen2.5-1.5B Q8_0 legacy completion serving. This follows the CLI throughput
slice that measured the same model above 10 tok/s locally, but HTTP throughput
for this model remains unmeasured.
