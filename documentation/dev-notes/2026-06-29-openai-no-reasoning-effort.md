# OpenAI No-Reasoning Effort

Date: 2026-06-29

## Slice

Ferrite's OpenAI-compatible chat endpoint now accepts `reasoning_effort: "none"`
as an explicit no-op. The official Chat Completions API documents
`reasoning_effort` as an optional field for reasoning models with supported
values including `none`, `minimal`, `low`, `medium`, `high`, and `xhigh`.
Ferrite accepts only the disabled form because the current local inference path
does not implement reasoning-effort behavior.

Source reference:

- https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create

## Implementation

- Added `crates/ferrite-server/src/openai/schema/reasoning_effort.rs` to keep
  reasoning-effort compatibility validation separate from chat request logic.
- Updated chat unsupported-field detection to accept missing `reasoning_effort`
  or `reasoning_effort: "none"`.
- Added a fixture-backed route test for explicit no-reasoning effort.
- Added a route-level rejection regression for enabled reasoning-effort values.

## Red Test

```sh
cargo test -p ferrite-server chat_endpoint_accepts_no_reasoning_effort -- --nocapture
```

Failed before implementation with:

```text
unsupported chat completion field(s): reasoning_effort
```

## Validation

```sh
cargo fmt --all -- --check
cargo test -p ferrite-server chat_endpoint_accepts_no_reasoning_effort -- --nocapture
cargo test -p ferrite-server openai::schema::reasoning_effort -- --nocapture
cargo test -p ferrite-server reasoning_effort -- --nocapture
```

All commands passed after implementation.

## Limits

This slice does not implement reasoning-token budgeting, hidden
chain-of-thought generation, model-specific reasoning effort, or any generation
behavior tied to `reasoning_effort`. Values such as `minimal`, `low`, `medium`,
`high`, and `xhigh` remain unsupported.
