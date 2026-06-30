# OpenAI Throughput Stop Option

Date: 2026-06-30

## Context

The Tier 1 OpenAI long-chat gate requires a stop/EOS variant for each required
model. The throughput client can now record streamed finish reasons, but it did
not have a way to send an explicit OpenAI `stop` sequence for the stop variant.

## Change

- Added `--stop STOP` to `ferrite-openai-throughput`.
- Rejected empty stop sequences.
- Added `"stop": "<sequence>"` to both legacy completion and chat completion
  request bodies when configured.

## Validation

RED:

```text
cargo test -p ferrite-server --lib throughput_client::tests -- --nocapture
error[E0599]: no method named `stop` found for struct
`throughput_client::config::ThroughputClientConfig`
```

GREEN:

```text
cargo test -p ferrite-server --lib throughput_client::tests -- --nocapture
test result: ok. 35 passed; 0 failed; 0 ignored; 0 measured; 303 filtered out
```

## Limits

- This slice only lets the benchmark client send one string stop sequence.
- It does not run a real stop/EOS benchmark.
- It does not yet support an array of OpenAI stop sequences.
