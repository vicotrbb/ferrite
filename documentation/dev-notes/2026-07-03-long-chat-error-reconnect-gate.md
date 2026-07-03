# Long-Chat Error Reconnect Gate Hardening

Date: 2026-07-03

## Slice

Strengthen the OpenAI long-chat proof gate's request-error reconnect evidence.

Before this slice, the disconnect probe required the reconnect stream to emit
generated content, but the error probe accepted any `200 OK` SSE reconnect that
ended with `[DONE]`. That was too weak for the dedicated long-chat milestone:
an authorization failure followed by a done-only reconnect would prove that the
server could answer, but not that the next valid request actually started a new
generation.

## Change

Commit `a3d6e49` extends `LongChatErrorProbeResult` with:

- `reconnect_generated_event`;
- `reconnect_started_new_generation`.

The error probe now rejects reconnect responses that contain only terminal SSE
events. The integrated long-chat summary also requires
`long_chat_summary_error_probe_reconnect_started_new_generation=true` before
`long_chat_summary_run_complete=true` can be emitted when `--error-probe` is
enabled.

This makes error recovery evidence match the stronger disconnect/reconnect
gate: both probes now require a generated stream event after the disruption.

## Validation

Red check:

```text
cargo test -p ferrite-server --test long_chat_gate -- --nocapture
```

The test target failed because `LongChatErrorProbeResult::new` did not yet
accept a generated-reconnect field.

Green checks:

```text
cargo fmt -- --check
cargo test -p ferrite-server error_probe -- --nocapture
cargo test -p ferrite-server --test long_chat_gate -- --nocapture
git diff --check
```

Results:

- parser-level error-probe tests: 2 passed;
- long-chat gate integration tests: 54 passed;
- formatting and whitespace checks passed.

## Remaining Proof Work

This is gate hardening, not a fresh real-model run. The next proof run should
execute the long-chat gate with `--error-probe`, `--disconnect-probe`, RSS
sampling, lifecycle server logs, and the 256/512/1024 token ladder. The
accepted benchmark note should preserve the new
`long_chat_error_probe_reconnect_generated_event=true` and
`long_chat_summary_error_probe_reconnect_started_new_generation=true` fields.
