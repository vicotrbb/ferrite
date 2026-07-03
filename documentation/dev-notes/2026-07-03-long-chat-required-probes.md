# Long-Chat Required Probes

Date: 2026-07-03

## Goal

Make the dedicated OpenAI long-chat gate able to reject closure attempts that
omit required operational probes.

Partial runs without reconnect or error probes remain useful evidence, but they
should not be able to look like closure for the dedicated long-chat milestone.

## Change

`ferrite-openai-long-chat-gate` now accepts:

```text
--require-probes error,disconnect,queue
```

Allowed values are:

- `error`
- `disconnect`
- `queue`

When configured, the plan emits:

```text
long_chat_required_probes=error,disconnect,queue
```

The final summary emits:

```text
long_chat_summary_required_probes=error,disconnect,queue
long_chat_summary_required_probes_completed=true|false
```

`long_chat_summary_run_complete=true` now requires the configured probes to
complete with their existing acceptance criteria.

## Validation

Red test evidence:

```text
cargo test -p ferrite-server --test long_chat_gate required_probes_make_summary_incomplete_when_probe_set_is_partial -- --nocapture
error[E0599]: no method named `required_probes` found for struct `LongChatGateConfig`
```

Green test evidence:

```text
cargo test -p ferrite-server --test long_chat_gate required_probes_make_summary_incomplete_when_probe_set_is_partial -- --nocapture
test required_probes_make_summary_incomplete_when_probe_set_is_partial ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 68 filtered out
```

Full long-chat gate test target:

```text
cargo test -p ferrite-server --test long_chat_gate -- --nocapture
```

Formatting:

```text
cargo fmt -- --check
git diff --check
```

## Limits

This is harness acceptance logic. It does not execute reconnect, error, or
queue probes by itself. Closure commands must still include the matching probe
execution flags, such as `--error-probe`, `--disconnect-probe`, and
`--queue-probe`.
