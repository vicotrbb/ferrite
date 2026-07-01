# OpenAI Long-Chat Disconnect Reconnect Status

## Context

The Tier 1 OpenAI long-chat gate requires disconnect behavior to be explicit:
Ferrite does not resume abandoned SSE generations, so a reconnect probe must
show bounded cleanup and a fresh follow-up generation.

Previous harness output recorded that a stream was aborted after generated
content and that a follow-up request completed. It did not explicitly record
that the reconnect response itself produced generated content, and the reconnect
validator accepted a done-only SSE response.

## Change

`ferrite-openai-long-chat-gate` now requires the disconnect reconnect response
to include a generated stream event before accepting it as completed. The
disconnect report now emits:

- `long_chat_disconnect_probe_reconnect_generated_event`;
- `long_chat_disconnect_probe_reconnect_started_new_generation`.

The integrated summary now emits
`long_chat_summary_disconnect_probe_reconnect_started_new_generation`, and
`long_chat_summary_run_complete=true` requires that field when the disconnect
probe is requested.

## Validation

```sh
cargo test -p ferrite-server --lib long_chat_gate::disconnect_probe -- --nocapture
cargo test -p ferrite-server --test long_chat_gate -- --nocapture
```

Results:

```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 348 filtered out
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Remaining Scope

This proves the harness no longer treats a done-only reconnect response as a
successful generation. It does not add resumable stream support; reconnects
remain fresh requests by design.
