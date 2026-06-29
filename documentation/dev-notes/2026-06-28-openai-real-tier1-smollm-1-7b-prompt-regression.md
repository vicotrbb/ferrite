# 2026-06-28 OpenAI Real Tier 1 SmolLM2 1.7B Prompt Regression

## Slice

This slice adds an opt-in real-model OpenAI-compatible HTTP regression for
SmolLM2-1.7B-Instruct Q4_K_M.

The existing real Tier 1 OpenAI prompt regressions covered Qwen2.5-1.5B Q8_0
and Q6_K. This test broadens the HTTP path to the largest local Llama-family
Tier 1 artifact already covered by CLI reference profiles.

## Test Added

```text
crates/ferrite-server/tests/openai_real_tier1_smollm_1_7b_prompts.rs
```

The test:

- starts Ferrite's OpenAI-compatible server with
  `target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf`;
- sends six `POST /v1/completions` requests;
- requests one generated token per prompt;
- verifies the OpenAI-shaped completion response;
- checks prompt token counts and completion usage.

The prompts are:

- `hello world`
- `The capital of France is`
- `Once upon a time`
- `Rust is a systems programming language`
- `Machine learning models can`
- `The recipe calls for`

## Expected First Tokens

| Prompt | Prompt tokens | First completion text |
| --- | ---: | --- |
| `hello world` | 2 | `"` |
| `The capital of France is` | 5 | ` Paris` |
| `Once upon a time` | 4 | `,` |
| `Rust is a systems programming language` | 7 | ` that` |
| `Machine learning models can` | 4 | ` also` |
| `The recipe calls for` | 4 | ` ` |

## Validation

Compile-only default ignored test run:

```sh
cargo test -p ferrite-server --test openai_real_tier1_smollm_1_7b_prompts
```

Result:

```text
test live_http_server_matches_smollm_1_7b_q4_first_tokens_for_reference_prompts ... ignored, requires local SmolLM2-1.7B Q4_K_M GGUF model artifact
test result: ok. 0 passed; 0 failed; 1 ignored
```

Real local model run:

```sh
cargo test -p ferrite-server --test openai_real_tier1_smollm_1_7b_prompts live_http_server_matches_smollm_1_7b_q4_first_tokens_for_reference_prompts -- --ignored --nocapture
```

Result:

```text
test live_http_server_matches_smollm_1_7b_q4_first_tokens_for_reference_prompts ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; finished in 209.09s
```

## Boundary

This proves the local OpenAI-compatible legacy completions path can drive the
real SmolLM2-1.7B Q4_K_M model for six deterministic one-token prompt cases.
It does not prove streaming SmolLM2 HTTP behavior, HTTP throughput, broad
concurrent serving, or full Tier 1 completion.
