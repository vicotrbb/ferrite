# OpenAI Long-Chat Module Split

## Context

The long-chat gate module had grown to hold config parsing, scenario expansion,
report formatting, and throughput bridging. Before adding the async runner, it
needed a cleaner module boundary so the next execution code does not turn the
gate into a large mixed-concern file.

## Change

- Kept `long_chat_gate.rs` as a small facade.
- Moved config parsing and validation into `long_chat_gate/config.rs`.
- Moved scenario data into `long_chat_gate/scenario.rs`.
- Moved report formatting into `long_chat_gate/report.rs`.
- Kept throughput conversion in `long_chat_gate/throughput.rs`.

## Validation

The existing long-chat integration target passed before and after the split:
`cargo test -p ferrite-server --test long_chat_gate -- --nocapture`.

## Limits

This is a behavior-preserving organization slice. It does not execute real
streaming requests, collect RSS samples, verify stop/EOS behavior, or exercise
reconnect/error behavior.
