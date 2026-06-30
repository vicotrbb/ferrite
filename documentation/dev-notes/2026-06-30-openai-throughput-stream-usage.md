# OpenAI Throughput Stream Usage Option

Date: 2026-06-30

## Context

The Tier 1 OpenAI long-chat gate requires streamed chat runs to record usage
when requested. Ferrite's OpenAI server already supports
`stream_options.include_usage`, but the release-oriented
`ferrite-openai-throughput` client could not request that field.

## Change

- Added `--stream-usage` to the throughput client.
- Rejected `--stream-usage` unless `--stream` is also present, matching the
  server-side OpenAI compatibility validation.
- Added `stream_options: {"include_usage": true}` to both legacy completion and
  chat completion request bodies when `--stream-usage` is set.

## Validation

RED:

```text
cargo test -p ferrite-server --lib throughput_client::tests -- --nocapture
error[E0599]: no method named `stream_usage` found for struct
`throughput_client::config::ThroughputClientConfig`
```

GREEN:

```text
cargo test -p ferrite-server --lib throughput_client::tests -- --nocapture
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 303 filtered out
```

## Limits

- This slice only enables the client to request streamed usage chunks.
- It does not run the 256, 512, or 1024-token long-chat gate.
- It does not validate streamed usage values inside the throughput client yet;
  recorded benchmark evidence must still inspect usage in the response stream.
