# 2026-06-28 OpenAI Real Tier 1 Qwen 1.5B Q6 Prompt Regression

## Scope

This slice adds an ignored real-model HTTP regression for six fixed
Qwen2.5-1.5B Q6_K prompt profiles through `POST /v1/completions`.

The test is isolated in
`crates/ferrite-server/tests/openai_real_tier1_qwen_1_5b_q6_prompts.rs` so Q6_K
prompt coverage remains separate from Q8_0 prompt coverage, endpoint-shape
coverage, and queue-order coverage.

## Test Shape

- Model: `target/models/qwen2.5-1.5b-instruct-q6_k.gguf`
- Model id: `qwen2.5-1.5b-q6_k`
- Endpoint: `POST /v1/completions`
- Generation limit: 1 token per prompt
- Prompts:
  - `hello world`
  - `The capital of France is`
  - `Once upon a time`
  - `Rust is a systems programming language`
  - `Machine learning models can`
  - `The recipe calls for`

Each response asserts HTTP `200`, the OpenAI `text_completion` object shape,
the configured model id, the decoded first-token text, and prompt/completion
usage counts.

## Verification

```sh
cargo fmt --check
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_q6_prompts live_http_server_matches_qwen_1_5b_q6_first_tokens_for_reference_prompts -- --ignored --nocapture
```

Result:

```text
test live_http_server_matches_qwen_1_5b_q6_first_tokens_for_reference_prompts ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 286.58s
```

## Interpretation

This narrows the broader model/prompt behavior gap for Ferrite's
OpenAI-compatible server by proving six deterministic Qwen2.5-1.5B Q6_K
reference prompts through the HTTP legacy completion path.

It does not prove broad prompt behavior, chat prompt parity for these six
profiles, longer completions through HTTP, x86_64 HTTP behavior, queue
fairness, or server throughput.
