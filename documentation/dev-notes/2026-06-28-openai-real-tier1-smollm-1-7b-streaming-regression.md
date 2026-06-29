# 2026-06-28 OpenAI Real Tier 1 SmolLM2 1.7B Streaming Regression

## Slice

This slice adds an opt-in real-model OpenAI-compatible HTTP streaming
regression for SmolLM2-1.7B-Instruct Q4_K_M.

The existing SmolLM2 HTTP prompt regression covered non-streaming legacy
completions. This test exercises the same six deterministic prompts through
`POST /v1/completions` with `stream: true`.

## Test Added

```text
crates/ferrite-server/tests/openai_real_tier1_smollm_1_7b_streaming.rs
```

The test:

- starts Ferrite's OpenAI-compatible server with
  `target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf`;
- sends six streaming `POST /v1/completions` requests;
- requests one generated token per prompt;
- verifies HTTP `200` and `text/event-stream`;
- parses SSE `data:` JSON events;
- checks the generated token chunk, terminal stop chunk, and `[DONE]`.

The prompts are:

- `hello world`
- `The capital of France is`
- `Once upon a time`
- `Rust is a systems programming language`
- `Machine learning models can`
- `The recipe calls for`

## Expected First Tokens

| Prompt | First completion text |
| --- | --- |
| `hello world` | `"` |
| `The capital of France is` | ` Paris` |
| `Once upon a time` | `,` |
| `Rust is a systems programming language` | ` that` |
| `Machine learning models can` | ` also` |
| `The recipe calls for` | ` ` |

## Validation

Compile-only default ignored test run:

```sh
cargo test -p ferrite-server --test openai_real_tier1_smollm_1_7b_streaming
```

Result:

```text
test live_http_server_streams_smollm_1_7b_q4_first_tokens_for_reference_prompts ... ignored, requires local SmolLM2-1.7B Q4_K_M GGUF model artifact
test result: ok. 0 passed; 0 failed; 1 ignored
```

Real local model run:

```sh
cargo test -p ferrite-server --test openai_real_tier1_smollm_1_7b_streaming live_http_server_streams_smollm_1_7b_q4_first_tokens_for_reference_prompts -- --ignored --nocapture
```

Result:

```text
test live_http_server_streams_smollm_1_7b_q4_first_tokens_for_reference_prompts ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; finished in 210.95s
```

## Boundary

This proves the local OpenAI-compatible streaming legacy completions path can
drive the real SmolLM2-1.7B Q4_K_M model for six deterministic one-token prompt
cases. It does not prove SmolLM2 chat HTTP behavior, HTTP throughput, broad
concurrent serving, or full Tier 1 completion.
