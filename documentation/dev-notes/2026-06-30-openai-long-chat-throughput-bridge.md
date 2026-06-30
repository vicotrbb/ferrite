# OpenAI Long-Chat Throughput Bridge

## Context

The long-chat gate now enumerates model-aware scenarios, but those scenarios
still needed a direct path into the existing OpenAI throughput client. Without
that bridge, the gate command could list work but not safely reuse the tested
request builder and streaming validators.

## Change

- Added long-chat target fields for server address, API key, prompt, assistant
  context, and follow-up text.
- Added parsing and validation for those fields.
- Added `long_chat_gate::throughput` as a focused module.
- Added `LongChatGateConfig::throughput_args()` to convert a scenario into a
  `ferrite-openai-throughput` argument vector.
- The generated throughput config uses chat completions, streaming,
  `stream_options.include_usage`, one request, concurrency one, the scenario
  model, and the scenario token length.

## RED

`cargo test -p ferrite-server --test long_chat_gate -- --nocapture` failed
because the long-chat config did not expose target fields or
`throughput_args()`.

## GREEN

The focused integration target passed 11 tests after adding the target fields
and throughput bridge.

## Limits

This slice builds validated throughput-client arguments. The long-chat command
still does not execute those arguments, run real model probes, collect RSS,
verify stop/EOS behavior, or exercise reconnect/error behavior.
