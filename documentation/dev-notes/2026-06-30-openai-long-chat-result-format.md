# OpenAI Long-Chat Result Format

## Context

The long-chat gate has a typed throughput plan, but the runner also needs a
stable evidence shape for each completed scenario. Without that shape, the
execution loop would have to invent ad hoc output while making network calls.

## Change

- Added `LongChatScenarioResult`.
- Added `format_scenario_result()`.
- The result format records model, turn, max tokens, completed request count,
  elapsed milliseconds, streaming finish reason, and streaming usage when
  present.

## RED

`cargo test -p ferrite-server --test long_chat_gate -- --nocapture` failed
because `LongChatScenarioResult` and `format_scenario_result()` did not exist.

## GREEN

The focused integration target passed 13 tests after adding the result module.

## Limits

This slice defines the per-scenario evidence format. It still does not execute
OpenAI-compatible streaming requests, collect RSS samples, verify stop/EOS
behavior, or exercise reconnect/error behavior.
