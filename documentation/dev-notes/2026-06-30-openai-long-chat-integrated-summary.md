# OpenAI Long-Chat Integrated Summary

## Slice

The long-chat gate already had executable full-length scenarios, explicit
finish-reason assertions, an unauthorized error probe, and a client disconnect
probe. Those outputs were still separate enough that a proof log required
manual interpretation to decide whether the same invocation included scenario
coverage, RSS/timing data, and reconnect/error behavior.

This slice adds a compact integrated summary to
`ferrite-openai-long-chat-gate`. When `--execute`, `--error-probe`, or
`--disconnect-probe` is used, the command now prints `long_chat_summary_*`
lines after the requested work completes.

The summary records:

- planned and completed scenario counts;
- whether every scenario had a terminal finish reason;
- whether usage accounting matched the terminal reason;
- whether per-token timing was present;
- whether RSS was required and present;
- whether the error probe was required and completed;
- whether the disconnect probe was required and completed;
- whether the current invocation is internally complete.

## Validation

```sh
cargo test -p ferrite-server --test long_chat_gate formats_integrated_long_chat_run_summary -- --nocapture
cargo test -p ferrite-server --test long_chat_gate -- --nocapture
CARGO_BUILD_JOBS=2 cargo clippy -p ferrite-server --all-targets -- -D warnings
```

Results:

- `formats_integrated_long_chat_run_summary`: passed after first failing on the
  missing `format_run_summary` API.
- Full `long_chat_gate` integration target: 21 passed.
- `ferrite-server` clippy across all targets: passed.

## Remaining Scope

This does not add resumable streams or a distinct EOS terminal reason. It makes
future long-chat proof logs easier to audit by tying scenario accounting,
RSS/timing, and requested reconnect/error probes into one final machine-readable
summary.
