# OpenAI Throughput Result Metadata

## Goal

Make `ferrite-openai-throughput` artifacts self-describing with non-secret run metadata.

## Context

The throughput harness already reported request counts, elapsed time, streaming timing, usage, finish reasons, and RSS samples. Its standalone output did not identify the target address, endpoint, model, token budget, configured request count, concurrency, or stream mode, which made saved artifacts harder to compare with long-chat gate output and future llama-benchy runs.

## Change

`format_result` now prefixes every result with:

```text
openai_http_addr=...
openai_http_endpoint=...
openai_http_model=...
openai_http_max_tokens=...
openai_http_configured_requests=...
openai_http_concurrency=...
openai_http_stream=...
openai_http_stream_usage=...
```

The metadata is intentionally limited to benchmark configuration values that are already non-secret and useful for reproducing a run.

## Red Test

The focused test failed before implementation because the result did not include metadata:

```text
left:  "openai_http_chat_completion_requests=2\nelapsed_ms=400\nrequests_per_second=5.000000"
right: "openai_http_addr=127.0.0.1:18080\nopenai_http_endpoint=/v1/chat/completions\nopenai_http_model=fixture-model\nopenai_http_max_tokens=16\nopenai_http_configured_requests=2\nopenai_http_concurrency=2\nopenai_http_stream=false\nopenai_http_stream_usage=false\nopenai_http_chat_completion_requests=2\nelapsed_ms=400\nrequests_per_second=5.000000"
```

## Validation

```text
CARGO_TARGET_DIR=target/codex-throughput-result-metadata cargo test -p ferrite-server throughput_client::tests -- --nocapture
```

Result: 46 passed, 0 failed.

```text
cargo fmt --all -- --check
git diff --check
```

Result: both exited cleanly.

## Limits

This slice only makes throughput result artifacts easier to audit and compare. It did not run a real model, start the OpenAI-compatible HTTP server, execute the long-chat gate, or run llama-benchy.
