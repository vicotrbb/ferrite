# OpenAI Long-Chat Scenario Expansion

## Context

The long-chat gate entrypoint could print the high-level plan, but it did not
yet expose concrete per-turn, per-token scenarios. The future runner needs
ordered scenario data so execution can be wired without duplicating plan logic
inside the CLI.

## Change

- Added `LongChatScenario`.
- Added `LongChatGateConfig::scenarios()`.
- Added `format_scenarios()` for stable per-scenario `key=value` output.
- Scenarios are turn-major: each turn runs every configured token length before
  moving to the next turn.

## RED

`cargo test -p ferrite-server --test long_chat_gate -- --nocapture` failed
because `format_scenarios` and `LongChatGateConfig::scenarios()` did not exist.

## GREEN

The focused integration test target passed 7 tests after adding ordered
scenario expansion and formatting.

## Limits

This slice still does not perform network requests or run a real model. It
prepares the executable scenario list that the next runner slice can use for
streaming chat probes, RSS sampling, stop/EOS variants, and reconnect/error
checks.
