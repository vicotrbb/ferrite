# OpenAI Long-Chat Model Scenarios

## Context

The Tier 1 long-chat gate is scoped per required model, per token length, and
per repeated turn. The initial gate plan counted only token lengths and turns,
which undercounted the real gate scope and could make the report look more
complete than the documented requirement.

## Change

- Added default required Tier 1 model IDs to `LongChatGateConfig`.
- Added `--models MODEL[,MODEL...]` for focused local runs.
- Updated planned scenario counts to `models * turns * token_lengths`.
- Updated ordered scenarios to be model-major, then turn-major, then token
  length.
- Updated report output to include `long_chat_models` and per-scenario model
  IDs.

## RED

`cargo test -p ferrite-server --test long_chat_gate -- --nocapture` failed
because `LongChatGateConfig::models()` and `LongChatScenario::model()` did not
exist.

## GREEN

The focused integration target passed 9 tests after adding model-aware scenario
expansion.

## Limits

This slice fixes the gate plan shape. It still does not perform OpenAI
streaming requests, real model probes, RSS sampling, stop/EOS variants, or
reconnect/error checks.
