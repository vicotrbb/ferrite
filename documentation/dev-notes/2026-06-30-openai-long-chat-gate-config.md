# OpenAI Long-Chat Gate Config

## Context

The Tier 1 OpenAI long-chat gate requires a dedicated proof shape for 256,
512, and 1024-token streaming chat responses plus repeated multi-turn
conversations. The existing throughput client can run streamed chat requests,
but the long-chat gate needs a focused configuration surface before execution
and reconnect checks are layered in.

## Change

- Added `ferrite_server::long_chat_gate`.
- Added `LongChatGateConfig` with default token lengths `256,512,1024`.
- Added a minimum `--turns` value of 4 to match the gate's repeated
  multi-turn requirement.
- Added parsing for custom comma-separated token lengths and turn counts.

## RED

`cargo test -p ferrite-server --test long_chat_gate -- --nocapture` failed
because `ferrite_server::long_chat_gate` did not exist.

## GREEN

The focused integration test target passed 4 tests for defaults, custom token
lengths, minimum turn validation, and empty token length rejection.

## Limits

This slice only establishes the long-chat gate configuration module. It does
not yet execute 256, 512, or 1024-token model probes, collect RSS samples,
drive reconnect/error checks, or write benchmark-result artifacts.
