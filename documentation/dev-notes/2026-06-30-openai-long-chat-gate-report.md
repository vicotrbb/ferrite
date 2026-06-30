# OpenAI Long-Chat Gate Report

## Context

The long-chat gate command could print the high-level plan, and the library
could expand ordered scenarios, but the CLI still did not expose those
scenarios to operators or benchmark notes.

## Change

- Added `format_report()`.
- Updated `ferrite-openai-long-chat-gate` to print the high-level plan followed
  by each ordered scenario line.
- The report output remains stable `key=value` text so shell logs and future
  benchmark artifacts can consume it directly.

## RED

`cargo test -p ferrite-server --test long_chat_gate -- --nocapture` failed
because `format_report` was not exported by `ferrite_server::long_chat_gate`.

## GREEN

The focused integration test target passed 8 tests after adding the combined
report formatter and switching the CLI to use it.

## Limits

The command is still not a probe runner. It enumerates the work that must run,
but does not yet perform OpenAI-compatible streaming requests, RSS sampling,
stop/EOS variants, or reconnect/error checks.
