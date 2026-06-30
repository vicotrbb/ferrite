# OpenAI Long-Chat Throughput Config Plan

## Context

The long-chat gate could build raw `ferrite-openai-throughput` arguments for a
single scenario. The next runner needs a typed, validated execution plan for
all scenarios so it can call the existing throughput client directly.

## Change

- Added `LongChatGateConfig::throughput_configs()`.
- The method expands every model/turn/token scenario and parses each generated
  argument vector through `ThroughputClientConfig::parse`.
- The resulting configs preserve scenario order and reuse the throughput
  client's validation for endpoint, streaming, usage, request count,
  concurrency, model, prompt, and token length.

## RED

`cargo test -p ferrite-server --test long_chat_gate -- --nocapture` failed
because `LongChatGateConfig::throughput_configs()` did not exist.

## GREEN

The focused integration target passed 12 tests after adding the typed
throughput config plan.

## Limits

This slice still does not execute HTTP requests. It prepares validated typed
configs that a follow-up runner can pass to the existing throughput benchmark
function.
