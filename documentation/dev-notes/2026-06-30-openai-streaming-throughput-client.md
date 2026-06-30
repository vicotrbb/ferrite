# OpenAI Streaming Throughput Client

Date: 2026-06-30

## Scope

This slice extends the release-oriented `ferrite-openai-throughput` benchmark
client with `--stream` so the same binary can measure OpenAI-compatible SSE
request-rate for:

- `POST /v1/completions`
- `POST /v1/chat/completions`

Streaming mode adds `"stream": true` to the request body and validates that the
HTTP response contains SSE `data:` events plus the terminal `data: [DONE]`
event before counting a request as complete.

This is benchmark infrastructure only. It does not by itself claim streaming
throughput for any model; release-build model results must still be recorded
under `documentation/benchmarks/`.

## Test-Driven Evidence

Red:

```text
cargo test -p ferrite-server --lib throughput_client -- --nocapture
error[E0599]: no method named `stream` found for struct `ThroughputClientConfig`
error[E0061]: this function takes 2 arguments but 3 arguments were supplied
```

Green:

```text
cargo test -p ferrite-server --lib throughput_client -- --nocapture
test throughput_client::tests::parses_chat_completion_benchmark_config ... ok
test throughput_client::tests::parses_minimal_completion_benchmark_config ... ok
test throughput_client::tests::formats_chat_completion_result_metric_name ... ok
test throughput_client::tests::formats_streaming_chat_completion_result_metric_name ... ok
test throughput_client::tests::builds_openai_compatible_streaming_chat_completion_request_body ... ok
test throughput_client::tests::builds_openai_compatible_chat_completion_request_body ... ok
test throughput_client::tests::builds_openai_compatible_streaming_completion_request_body ... ok
test throughput_client::tests::builds_openai_compatible_completion_request_body ... ok
test throughput_client::tests::parses_streaming_benchmark_config ... ok
test throughput_client::tests::validates_streaming_response_done_event ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 263 filtered out
```

Binary target compile check:

```text
cargo test -p ferrite-server --bin ferrite-openai-throughput
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## What Changed

- Added `--stream` parsing to the throughput client config.
- Added streaming request-body construction for legacy completions and chat
  completions.
- Added SSE response validation requiring at least one `data:` event and the
  terminal `data: [DONE]`.
- Added stream-specific metric names:
  `openai_http_streaming_completion_requests` and
  `openai_http_streaming_chat_completion_requests`.
- Updated `README.md` throughput-client usage.

## Remaining Work

Run release-build streaming throughput measurements against real Tier 1 models
and record the results under `documentation/benchmarks/`. Longer generations,
x86_64 behavior, steady-state load, and memory sampling remain separate proof
obligations.
