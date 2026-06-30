# OpenAI long-chat timing output

## Context

The long-chat gate needs the benchmark client to print per-token streaming
latency metrics. The previous slice added `StreamingTimingSummary`, but
`ThroughputResult` could not carry or format that summary.

## Slice

Add optional streaming timing output to the throughput result formatter:

- `streaming_token_events`
- `streaming_time_to_first_token_ms`
- `streaming_total_elapsed_ms`
- `streaming_tokens_per_second`
- `streaming_token_latency_min_ms`
- `streaming_token_latency_p50_ms`
- `streaming_token_latency_p95_ms`
- `streaming_token_latency_max_ms`

Non-streaming and untimed runs continue to print the existing request-count,
elapsed, and request-rate metrics only.

## TDD

RED:

- `cargo test -p ferrite-server --lib formats_streaming_timing_summary -- --nocapture`

Observed failure:

- compile failed because `ThroughputResult` had no `streaming_timing` field.

GREEN:

- Added `streaming_timing: Option<StreamingTimingSummary>` to
  `ThroughputResult`.
- Printed timing metrics only when the summary is present.
- Kept existing result-format tests explicit with `streaming_timing: None`.

## Validation

Executed:

- `cargo test -p ferrite-server --lib formats_streaming_timing_summary -- --nocapture`

Result:

- 1 passed, 0 failed.

This slice does not yet collect SSE event arrival offsets from the TCP reader.
It only establishes the output contract that the long-chat harness will emit
once incremental streaming timing collection is wired in.
