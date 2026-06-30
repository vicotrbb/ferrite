# OpenAI Throughput Stream Usage Output

Date: 2026-06-30

## Context

The long-chat gate needs streamed usage values in the benchmark evidence:
prompt tokens, completion tokens, and total tokens. The throughput client could
request `stream_options.include_usage`, but it did not extract the usage chunk
or print it in the benchmark output.

## Change

- Added a focused `streaming_usage` module for parsing streamed OpenAI usage
  chunks from SSE bodies.
- Added `StreamingUsageSummary` to the throughput result model.
- Parsed streamed usage from completed HTTP responses.
- Printed usage metrics when present:
  - `streaming_usage_prompt_tokens`
  - `streaming_usage_completion_tokens`
  - `streaming_usage_total_tokens`

## Validation

RED:

```text
cargo test -p ferrite-server --lib throughput_client::tests -- --nocapture
error[E0560]: struct `throughput_client::ThroughputResult` has no field named
`streaming_usage`
error[E0433]: cannot find type `StreamingUsageSummary` in this scope
```

GREEN:

```text
cargo test -p ferrite-server --lib throughput_client::tests -- --nocapture
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 303 filtered out
```

## Limits

- This slice extracts the first valid usage chunk from the completed SSE body.
- It does not yet fail a benchmark run when requested usage is absent.
- It does not prove any 256, 512, or 1024-token real-model long-chat run.
