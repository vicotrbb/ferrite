# OpenAI Streaming Timing Reader

Date: 2026-06-30

## Context

The Tier 1 OpenAI-compatible proof path needs a dedicated long-chat gate with
256, 512, and 1024-token streaming responses, repeated multi-turn
conversations, RSS before/after sampling, latency per token, stop/EOS behavior,
and client reconnect/error behavior.

The throughput client already accepted `--stream` and printed timing fields when
given a `StreamingTimingSummary`, but it did not derive that summary from the
actual HTTP/SSE response stream.

## Change

- Added an incremental SSE response tracker to the throughput HTTP client.
- Recorded token timing only after a complete SSE event delimiter is observed.
- Counted only generated token events:
  - chat `choices[].delta.content` values that are non-empty strings.
  - completion `choices[].text` values that are non-empty strings.
- Ignored role-only events, empty terminal chunks, malformed partial chunks, and
  `data: [DONE]`.
- Captured token event offsets at response-read granularity and exposed the
  first observed streamed request summary through `ThroughputResult`.

## Validation

RED:

```text
cargo test -p ferrite-server --lib throughput_client::tests -- --nocapture
error[E0425]: cannot find function `streaming_timing_from_response_snapshots` in module `http`
```

GREEN:

```text
cargo test -p ferrite-server --lib throughput_client::tests -- --nocapture
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 303 filtered out
```

## Limits

- Timing is measured at `TcpStream::read` chunk granularity. If several SSE
  token events arrive in one read, they share that offset.
- Multi-request streaming timing aggregation is intentionally not solved yet;
  the current benchmark result reports the first streamed request summary.
- The long-chat gate still needs real 256, 512, and 1024-token runs, RSS
  sampling, repeated multi-turn conversations, stop/EOS checks, and reconnect
  or error behavior checks.
