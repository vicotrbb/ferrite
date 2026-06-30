# OpenAI Throughput Token Count Validation

## Context

The long-chat proof gate needs streamed 256, 512, and 1024-token runs to prove
they actually reach the requested completion budget unless the model ends early
with EOS or a configured stop sequence.

## Change

- Added throughput-client validation for streamed responses when
  `--stream-usage` is enabled.
- If the stream finish reason is `length`, `completion_tokens` must equal the
  requested `--max-tokens` value.
- If the stream finish reason is `stop`, shorter completions remain valid.

## RED

`cargo test -p ferrite-server --lib throughput_client::tests -- --nocapture`
failed because `validate_streaming_token_count` did not exist.

## Limits

This slice validates the proof harness. It does not run a real model, and it
uses the already-parsed OpenAI-compatible finish reason to distinguish length
exhaustion from stop or EOS-style early termination.
