# OpenAI Chat Throughput Client

Date: 2026-06-30

## Scope

This slice extends the release-oriented `ferrite-openai-throughput` benchmark
client so the same binary can measure either:

- `POST /v1/completions`
- `POST /v1/chat/completions`

The new `--endpoint chat-completions` mode wraps the prompt as one user chat
message and validates OpenAI-compatible chat completion response shape before
counting a request as complete.

This is benchmark infrastructure only. It does not by itself claim chat
throughput for any model; release-build model results must still be recorded
under `documentation/benchmarks/`.

## Test-Driven Evidence

Red:

```text
cargo test -p ferrite-server throughput_client -- --nocapture
error[E0599]: no method named `endpoint` found for struct `ThroughputClientConfig`
error[E0433]: use of undeclared type `OpenAiEndpoint`
error[E0425]: cannot find function `request_body` in this scope
error[E0061]: this function takes 1 argument but 2 arguments were supplied
```

Green:

```text
cargo test -p ferrite-server throughput_client -- --nocapture
test throughput_client::tests::parses_chat_completion_benchmark_config ... ok
test throughput_client::tests::parses_minimal_completion_benchmark_config ... ok
test throughput_client::tests::formats_chat_completion_result_metric_name ... ok
test throughput_client::tests::builds_openai_compatible_completion_request_body ... ok
test throughput_client::tests::builds_openai_compatible_chat_completion_request_body ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 263 filtered out
```

The same command exited with status 0 after Cargo enumerated the remaining
filtered package test binaries.

## What Changed

- Added `OpenAiEndpoint` parsing to the throughput client config.
- Added `--endpoint completions|chat-completions`.
- Reused the same benchmark loop for both endpoint paths.
- Added chat request-body construction:
  `{"model":"...","messages":[{"role":"user","content":"..."}],"max_tokens":N}`.
- Added chat response validation for `object == "chat.completion"` and a string
  `choices[0].message.content`.
- Added endpoint-specific metric names:
  `openai_http_completion_requests` and
  `openai_http_chat_completion_requests`.
- Updated `README.md` throughput-client usage.

## Remaining Work

Run release-build chat throughput measurements against real Tier 1 models and
record the results under `documentation/benchmarks/`. Streaming throughput,
longer generations, x86_64 behavior, steady-state load, and memory sampling
remain separate proof obligations.
