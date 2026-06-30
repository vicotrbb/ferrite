# OpenAI long-chat streaming timing summary

## Context

The Tier 1 OpenAI long-chat gate requires per-token latency summaries for
streaming chat responses. The existing `ferrite-openai-throughput` client only
reports request-level elapsed time and requests per second, so the first harness
slice needs reusable timing math before the network reader is changed to record
SSE event arrival offsets.

## Slice

Add `StreamingTimingSummary` under `crates/ferrite-server/src/throughput_client/`
to summarize token-event arrival offsets:

- token event count;
- time to first token;
- total elapsed time;
- min, p50, p95, and max token-arrival latency;
- average generated tokens per second.

The summary treats the first token latency as time from request start to first
token event, and later token latencies as deltas between token events.

## TDD

RED:

- `cargo test -p ferrite-server --lib summarizes_streaming_token_arrival_latencies -- --nocapture`

Observed failure:

- compile failed because `StreamingTimingSummary` did not exist.

GREEN:

- Added the focused streaming metrics module.
- Exported `StreamingTimingSummary` from `throughput_client`.

## Validation

Executed:

- `cargo test -p ferrite-server --lib summarizes_streaming_token_arrival_latencies -- --nocapture`

Result:

- 1 passed, 0 failed.

This is harness foundation only. It does not yet read SSE events incrementally,
sample RSS, run 256/512/1024-token probes, or record long-chat benchmark
evidence.
