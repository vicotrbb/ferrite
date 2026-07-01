# OpenAI Long-Chat Disconnect Timeout

## Context

The integrated SmolLM2 1.7B stop proof initially failed during the disconnect
probe. The probe successfully closed a streaming client after a generated event,
but the immediate reconnect loop kept receiving retryable `429` responses and
gave up after the previous fixed five-second retry window.

The server behavior was bounded: the follow-up reconnect was rejected while the
single inference permit was still occupied. The harness timeout was too short
for slower real-model cleanup after abandoning an eight-token stream.

## Change

`ferrite-openai-long-chat-gate` now exposes:

```text
--disconnect-reconnect-timeout-ms 30000
```

The default reconnect timeout is 30 seconds. The disconnect probe retries
retryable `429` responses until that deadline and still fails closed if the
server does not accept a new streaming request within the configured window.

## Validation

```sh
cargo test -p ferrite-server --test long_chat_gate -- --nocapture
CARGO_BUILD_JOBS=2 cargo clippy -p ferrite-server --all-targets -- -D warnings
git diff --check
```

Observed results:

- `long_chat_gate`: 21 tests passed.
- `cargo clippy -p ferrite-server --all-targets -- -D warnings`: passed.
- `git diff --check`: passed.

The follow-up SmolLM integrated stop gate then completed with:

```text
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
long_chat_summary_run_complete=true
```

## Scope

This is a harness operability fix. It does not add resumable SSE streams and
does not change server queue semantics. A client disconnect still starts any
retry as a new generation after the previous request releases the inference
permit.
