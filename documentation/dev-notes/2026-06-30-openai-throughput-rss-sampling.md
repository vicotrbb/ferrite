# OpenAI Throughput RSS Sampling

Date: 2026-06-30

## Context

The Tier 1 OpenAI long-chat gate requires raw server RSS byte samples before
and after long streaming runs, plus a two-second idle sample. The throughput
client already records request and streaming timing metrics, but it did not
sample server memory.

## Change

- Added `--rss-pid PID` to `ferrite-openai-throughput`.
- Added `--rss-idle-ms` with a default two-second delay.
- Added a focused RSS module that samples local process RSS with:

```text
ps -o rss= -p <pid>
```

- Converted `ps` KiB output to raw bytes.
- Printed RSS metrics when a PID is supplied:
  - `server_rss_before_bytes`
  - `server_rss_after_bytes`
  - `server_rss_idle_bytes`

## Validation

RED:

```text
cargo test -p ferrite-server --lib throughput_client::tests -- --nocapture
error[E0599]: no method named `rss_pid` found for struct
`throughput_client::config::ThroughputClientConfig`
error[E0560]: struct `throughput_client::ThroughputResult` has no field named
`rss`
error[E0433]: cannot find type `RssSummary` in this scope
```

GREEN:

```text
cargo test -p ferrite-server --lib throughput_client::tests -- --nocapture
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 303 filtered out
```

## Limits

- This is local PID RSS sampling only.
- It does not sample before model load because the throughput client attaches to
  an already running server.
- It does not collect homelab pod memory, CPU limits, or Kubernetes context
  evidence.
- It does not yet sample immediately after first token; timing collection
  already observes first token arrival, but RSS sampling is before, after, and
  after idle in this slice.
