# OpenAI Throughput Stream Validation

Date: 2026-06-30

## Context

The Tier 1 OpenAI long-chat gate requires streamed responses to prove an
SSE-compatible shape, exactly one terminal `[DONE]` marker, JSON data before
termination, and usage chunks when usage is requested. The throughput client
previously accepted weaker streamed responses: any `data:` line plus at least
one `[DONE]` marker.

## Change

- Added stricter streamed response validation to the throughput client:
  - requires `content-type: text/event-stream`;
  - requires exactly one `data: [DONE]` marker;
  - requires at least one JSON `data:` event before termination;
  - requires a parsed streaming usage chunk when `--stream-usage` is set.
- Wired benchmark response validation to the parsed `--stream-usage` config.

## Validation

RED:

```text
cargo test -p ferrite-server --lib throughput_client::tests -- --nocapture
error[E0061]: this function takes 3 arguments but 4 arguments were supplied
```

GREEN:

```text
cargo test -p ferrite-server --lib throughput_client::tests -- --nocapture
test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 303 filtered out
```

## Limits

- This slice strengthens the benchmark client validator only.
- It does not run the 256, 512, or 1024-token real-model long-chat gate.
- It does not inspect generated text for dropped or reordered byte-level BPE
  fragments; that still requires real-model benchmark evidence.
