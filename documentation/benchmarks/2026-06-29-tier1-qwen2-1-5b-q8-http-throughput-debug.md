# Tier 1 Qwen2.5-1.5B Q8_0 HTTP Throughput Harness Check

Date: 2026-06-29

## Scope

This is a bounded OpenAI-compatible HTTP throughput harness check for
Qwen2.5-1.5B-Instruct Q8_0 using Ferrite's ignored real-model integration test.
It measures three one-token legacy completion requests through
`POST /v1/completions` after starting the local Ferrite test server.

This is not a release throughput pass. The commands run through Cargo's default
test profile, use a single local client shape, use one prompt, and request one
generated token per HTTP request. The result proves the real-model measurement
harness can execute and record bounded sequential and queued request-rate
samples. It does not prove production throughput, release-build throughput,
multi-client serving, long-running steady state, batching, or full Tier 1 HTTP
throughput.

## Model

- Model: Qwen2.5-1.5B-Instruct Q8_0 GGUF
- Path: `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
- Server model ID: `qwen2.5-1.5b-q8_0`
- Endpoint: `POST /v1/completions`
- Request body: `{"model":"qwen2.5-1.5b-q8_0","prompt":"hello world","max_tokens":1}`
- Expected response text: newline

## Sequential Request Check

Command:

```sh
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_throughput live_http_server_measures_qwen_1_5b_q8_sequential_completion_request_rate -- --ignored --nocapture
```

Result:

```text
qwen_1_5b_q8_sequential_http_completion_requests=3 elapsed_ms=81488 requests_per_second=0.036815
test live_http_server_measures_qwen_1_5b_q8_sequential_completion_request_rate ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 82.23s
```

## Queued Request Check

Command:

```sh
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_throughput live_http_server_measures_qwen_1_5b_q8_queued_completion_request_rate -- --ignored --nocapture
```

Result:

```text
qwen_1_5b_q8_queued_http_completion_requests=3 elapsed_ms=82187 requests_per_second=0.036502
test live_http_server_measures_qwen_1_5b_q8_queued_completion_request_rate ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 82.86s
```

## Interpretation

Both harness checks completed successfully and validated each HTTP response as
an OpenAI-shaped legacy completion from the real Qwen2.5-1.5B Q8_0 model. The
queued case used Ferrite's single-inference-permit server with a bounded wait
window, so it is a request-queue measurement rather than concurrent model
execution.

The measured request rates are low because this was a debug test-profile run.
A future throughput gate should use an explicit release-build benchmark
protocol with host details, request counts, warmup behavior, prompt set,
generated-token counts, client concurrency, and memory sampling.
