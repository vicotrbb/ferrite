# OpenAI Long-Chat Execute Flag

## Context

The long-chat gate command could print the scenario plan and the library had a
runner API, but the CLI still had no explicit execution switch. Running by
default would be risky because the full default gate covers 48 long streaming
requests.

## Change

- Added `--execute`.
- Kept plan-only output as the default.
- When `--execute` is present, `ferrite-openai-long-chat-gate` calls
  `LongChatGateConfig::run()` and prints each formatted scenario result.
- The CLI remains explicit about live execution while preserving the same
  report output before the run starts.

## RED

`cargo test -p ferrite-server --test long_chat_gate -- --nocapture` failed
because `LongChatGateConfig::execute()` did not exist.

## GREEN

The focused integration target passed 14 tests after adding execute-flag
parsing and the CLI execution branch.

## Limits

Verification did not pass `--execute`, so no real OpenAI-compatible server or
model was exercised in this slice. RSS sampling, stop/EOS behavior, and
reconnect/error behavior remain unproven.
