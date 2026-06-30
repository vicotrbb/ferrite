# OpenAI Long-Chat Result Timing And RSS

## Context

The long-chat result formatter emitted scenario identity, request count,
finish reason, and token usage. The gate also requires per-token latency and
RSS evidence, both of which are already available from the throughput client.

## Change

- Added streaming timing fields to `format_scenario_result()`.
- Added RSS before, after, and idle byte samples to `format_scenario_result()`.
- Kept fields conditional so plan or partial results without those summaries
  still format cleanly.

## RED

`cargo test -p ferrite-server --test long_chat_gate formats_long_chat_scenario_result -- --nocapture`
failed because timing and RSS fields were missing from the formatted result.

## GREEN

The focused formatter test passed after adding timing and RSS output.

## Limits

This slice only exposes already-collected summaries in the long-chat result
format. It does not run a real server, load a model, collect new RSS samples,
or verify reconnect/error behavior.
