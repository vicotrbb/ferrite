# OpenAI Long-Chat Gate Entrypoint

## Context

The long-chat gate needs a dedicated command path before it can become a full
probe runner. The previous slice added configuration parsing for required token
lengths and repeated turns, but there was not yet a callable entrypoint.

## Change

- Added `ferrite-openai-long-chat-gate`.
- Added `format_plan` for stable `key=value` plan output.
- The command currently prints token lengths, turn count, and total planned
  scenarios.

Example output:

```text
long_chat_token_lengths=256,512,1024
long_chat_turns=4
long_chat_planned_scenarios=12
```

## RED

`cargo test -p ferrite-server --test long_chat_gate -- --nocapture` failed
because `format_plan` was not exported by `ferrite_server::long_chat_gate`.

## GREEN

The focused integration test target passed 5 tests after adding the formatter
and entrypoint.

## Limits

The command is still plan-only. It does not start or probe a server, run real
256/512/1024-token streaming requests, sample RSS, verify stop/EOS behavior,
or exercise reconnect/error behavior yet.
