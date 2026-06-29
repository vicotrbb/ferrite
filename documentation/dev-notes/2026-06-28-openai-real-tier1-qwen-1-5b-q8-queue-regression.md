# 2026-06-28 OpenAI Real Tier 1 Qwen 1.5B Q8 Queue Regression

## Scope

This slice turns the retained local Qwen2.5-1.5B Q8_0 queue-order probe into an
ignored real-model integration regression.

The test is intentionally isolated in
`crates/ferrite-server/tests/openai_real_tier1_qwen_1_5b_queue.rs` so the
existing endpoint-shape test file stays focused.

## Test Shape

- Model: `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
- Model id: `qwen2.5-1.5b-q8_0`
- Wait window: 180 seconds
- Holder request: streaming chat, prompt `hello world`, 4 generated tokens
- Queued requests: two legacy completions, prompt `hello world`, 1 generated
  token each
- Start spacing: holder, then about 50 ms, then `queued_one`, then about 20 ms,
  then `queued_two`

The regression asserts:

- the holder returns HTTP `200` and emits `data: [DONE]`;
- both queued completions return HTTP `200`;
- both queued completions preserve the expected Qwen2.5-1.5B Q8_0 one-token
  response shape;
- the recorded completion order is `holder_stream`, `queued_one`,
  `queued_two`.

## Verification

```sh
cargo fmt --check
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_queue live_http_server_serves_qwen_1_5b_q8_wait_queue_in_start_order -- --ignored --nocapture
```

Result:

```text
test live_http_server_serves_qwen_1_5b_q8_wait_queue_in_start_order ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 163.88s
```

## Interpretation

This makes the three-request Qwen2.5-1.5B Q8_0 bounded-wait queue-order proof
reproducible through the Rust integration test harness.

It still does not prove general server throughput, load fairness, cancellation,
or broad long-stream overlap. It covers one local model, one prompt, one holder
stream, two queued requests, and one bounded wait policy.
