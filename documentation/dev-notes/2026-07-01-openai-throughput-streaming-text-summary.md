# OpenAI Throughput Streaming Text Summary

## Context

The long-chat gate needs to carry assistant output across turns. The throughput
client already extracted streaming finish reasons, timing, usage, and RSS
samples, but it did not retain assistant-visible generated text from SSE
responses.

## Change

The throughput client now extracts assistant-visible streaming text from both
OpenAI-compatible chat and legacy completion SSE chunks:

- chat streaming chunks use `choices[].delta.content`;
- legacy completion streaming chunks use `choices[].text`;
- role-only chunks, terminal finish chunks, usage chunks, and `[DONE]` are not
  included.

`ThroughputResult` now carries `streaming_text`, and formatted throughput output
reports `streaming_text_bytes` when generated text is present. This avoids
printing large generated text into benchmark summaries while still making the
presence and size of captured text machine-readable.

## Validation

```sh
cargo test -p ferrite-server throughput_client --lib -- --nocapture
```

Result:

```text
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 307 filtered out
```

## Remaining Scope

This slice only captures generated streaming text. The long-chat gate still
needs a follow-up slice to use that text as the assistant context for later
turns in each model and token-length sequence.
