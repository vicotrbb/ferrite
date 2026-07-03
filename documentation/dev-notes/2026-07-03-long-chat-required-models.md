# Long-Chat Required Models

Date: 2026-07-03

## Goal

Make the dedicated OpenAI long-chat gate able to reject partial model sets when
the proof milestone requires all Tier 1 HTTP model artifacts.

Partial single-model runs remain useful evidence, but they should not be able
to look like closure for the dedicated long-chat milestone.

## Change

`ferrite-openai-long-chat-gate` now accepts:

```text
--require-models Qwen2.5-0.5B-Instruct-Q4_K_M,Qwen2.5-1.5B-Instruct-Q8_0,Qwen2.5-1.5B-Instruct-Q6_K,SmolLM2-1.7B-Instruct-Q4_K_M
```

When configured, the plan emits:

```text
long_chat_required_models=...
```

The final summary emits:

```text
long_chat_summary_required_models=...
long_chat_summary_required_models_present=true|false
```

`long_chat_summary_run_complete=true` now requires the configured model ids to
appear in completed scenario results.

## Validation

Red test evidence:

```text
cargo test -p ferrite-server --test long_chat_gate required_models_make_summary_incomplete_when_model_set_is_partial -- --nocapture
error[E0599]: no method named `required_models` found for struct `LongChatGateConfig`
```

Green test evidence:

```text
cargo test -p ferrite-server --test long_chat_gate required_models_make_summary_incomplete_when_model_set_is_partial -- --nocapture
test required_models_make_summary_incomplete_when_model_set_is_partial ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 67 filtered out
```

Full long-chat gate test target:

```text
cargo test -p ferrite-server --test long_chat_gate -- --nocapture
test result: ok. 68 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Formatting and diff hygiene:

```text
cargo fmt -- --check
git diff --check
```

## Limits

This is harness acceptance logic. It does not execute the full Tier 1 model
matrix, and it does not prove any model result by itself.
