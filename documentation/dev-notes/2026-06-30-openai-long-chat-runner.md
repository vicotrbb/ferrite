# OpenAI Long-Chat Runner

## Context

The long-chat gate had a typed throughput plan and per-scenario result format,
but no runner that connected those two pieces. The next step toward live proof
is a runner API that iterates every scenario in order and wraps each throughput
result with its scenario identity.

## Change

- Added `long_chat_gate::runner`.
- Added `LongChatGateConfig::run_with_executor()` for deterministic tests and
  future controlled execution variants.
- Added `LongChatGateConfig::run()` as the real async path that calls the
  existing `run_completion_benchmark()` for each scenario's typed throughput
  config.
- Preserved model-major, turn-major, token-length scenario order.

## RED

`cargo test -p ferrite-server --test long_chat_gate -- --nocapture` failed
because `LongChatGateConfig::run_with_executor()` did not exist.

## GREEN

The focused integration target passed 14 tests after adding the runner module.

## Limits

This slice adds the executable runner API, but the verification used an
injected executor. It did not run a real OpenAI-compatible server, load a real
model, collect RSS samples, verify stop/EOS behavior, or exercise
reconnect/error behavior.
