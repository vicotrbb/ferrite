# OpenAI Long-Chat Probe Token Budget

## Scope

The Tier 1 OpenAI long-chat gate requires reconnect and error behavior to be
proven alongside longer streaming chat runs. The existing harness could execute
`--error-probe` and `--disconnect-probe`, but those probes used fixed short
request lengths: one token for the error reconnect request and eight tokens for
the disconnect request.

That was enough for short integrated stop summaries, but it left a gap for
combined long-chat evidence because a 256-token gate run could still report
passing reconnect/error probes that only exercised short reconnect generations.

## Change

`ferrite-openai-long-chat-gate` now accepts:

```sh
--probe-max-tokens TOKENS
```

When the option is present, both long-chat probes use that token budget:

- `--error-probe` sends the unauthorized request and the valid reconnect
  request with the configured `max_tokens`;
- `--disconnect-probe` aborts after the first generated SSE event, then retries
  a fresh streaming request with the configured `max_tokens`.

When the option is absent, existing short defaults are preserved:

- error reconnect request: `max_tokens=1`;
- disconnect request: `max_tokens=8`.

The formatted probe output now includes:

```text
long_chat_error_probe_max_tokens=...
long_chat_disconnect_probe_max_tokens=...
```

That keeps benchmark notes from treating a short probe as evidence for a longer
combined gate slice.

## Validation

Focused harness tests:

```sh
cargo test -p ferrite-server --test long_chat_gate -- --nocapture
```

Result:

```text
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Formatting check:

```sh
cargo fmt --all -- --check
```

Result: passed.

## Remaining Scope

This slice only adds and validates the harness capability. It does not by
itself prove a real 256, 512, or 1024-token reconnect/error run. The next proof
slice should run at least one real Tier 1 model with `--probe-max-tokens 256`
and record RSS, latency, finish reason, usage, and the new probe token-budget
fields.
