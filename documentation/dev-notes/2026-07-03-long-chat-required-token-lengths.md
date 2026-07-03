# Long-Chat Required Token Lengths

Date: 2026-07-03

## Goal

Make the dedicated OpenAI long-chat gate able to reject partial token-length
ladders when the proof milestone requires `256`, `512`, and `1024` completion
tokens.

Partial one-length runs remain useful evidence, but they should not be able to
look like closure for the dedicated long-chat milestone.

## Change

`ferrite-openai-long-chat-gate` now accepts:

```text
--require-token-lengths 256,512,1024
```

When configured, the plan emits:

```text
long_chat_required_token_lengths=256,512,1024
```

The final summary emits:

```text
long_chat_summary_required_token_lengths=256,512,1024
long_chat_summary_required_token_lengths_present=true|false
```

`long_chat_summary_run_complete=true` now requires the configured token lengths
to appear in completed scenario results.

## Validation

Red test evidence:

```text
cargo test -p ferrite-server --test long_chat_gate required_token_lengths_make_summary_incomplete_when_ladder_is_partial -- --nocapture
error[E0599]: no method named `required_token_lengths` found for struct `LongChatGateConfig`
```

Green test evidence:

```text
cargo test -p ferrite-server --test long_chat_gate required_token_lengths_make_summary_incomplete_when_ladder_is_partial -- --nocapture
test required_token_lengths_make_summary_incomplete_when_ladder_is_partial ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 66 filtered out
```

Full long-chat gate test target:

```text
cargo test -p ferrite-server --test long_chat_gate -- --nocapture
test result: ok. 67 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Formatting and diff hygiene:

```text
cargo fmt -- --check
git diff --check
```

## Limits

This is harness acceptance logic. It does not execute the 256/512/1024 proof by
itself, and it does not change generation, tokenization, stop/EOS behavior,
RSS sampling, or reconnect behavior.
