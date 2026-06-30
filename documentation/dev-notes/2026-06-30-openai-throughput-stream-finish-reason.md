# OpenAI Throughput Stream Finish Reason

Date: 2026-06-30

## Context

The Tier 1 OpenAI long-chat gate requires each streaming run to record whether
generation ended because it reached the requested length or stopped early via a
stop/EOS condition. The throughput client already validated SSE shape, timing,
usage, and RSS samples, but it did not extract terminal `finish_reason` values.

## Change

- Added a focused `streaming_finish` module.
- Parsed the first non-empty streamed `choices[].finish_reason` value from SSE
  bodies.
- Added `streaming_finish_reason` to formatted throughput output when present.
- Tightened streamed response validation to reject streams that terminate
  without any parsed `finish_reason`.

## Validation

RED:

```text
cargo test -p ferrite-server --lib throughput_client::tests -- --nocapture
error[E0560]: struct `throughput_client::ThroughputResult` has no field named
`streaming_finish`
error[E0433]: cannot find type `StreamingFinishSummary` in this scope
```

GREEN:

```text
cargo test -p ferrite-server --lib throughput_client::tests -- --nocapture
test result: ok. 31 passed; 0 failed; 0 ignored; 0 measured; 303 filtered out
```

## Limits

- This slice records finish reasons emitted by the server; it does not add the
  client-side `stop` request option yet.
- It does not run real 256, 512, or 1024-token model proof.
- It does not classify tokenizer EOS separately from OpenAI `"stop"`; the gate
  evidence must still inspect model output and request configuration.
