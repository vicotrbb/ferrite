# OpenAI Throughput Second-Turn Chat Shape

Date: 2026-06-30

## Context

The Tier 1 OpenAI long-chat gate requires repeated multi-turn conversations.
For each model, a later request must include the original user message, the
assistant response, and a second user follow-up. The throughput client could
only send a single user message in `POST /v1/chat/completions`.

## Change

- Added `--assistant-context TEXT`.
- Added `--follow-up TEXT`.
- Required both flags to be provided together.
- Restricted the flags to `--endpoint chat-completions`.
- Built the second-turn OpenAI message shape:

```json
[
  { "role": "user", "content": "<prompt>" },
  { "role": "assistant", "content": "<assistant-context>" },
  { "role": "user", "content": "<follow-up>" }
]
```

## Validation

RED:

```text
cargo test -p ferrite-server --lib throughput_client::tests -- --nocapture
error[E0599]: no method named `assistant_context` found for struct
`throughput_client::config::ThroughputClientConfig`
error[E0599]: no method named `follow_up` found for struct
`throughput_client::config::ThroughputClientConfig`
```

GREEN:

```text
cargo test -p ferrite-server --lib throughput_client::tests -- --nocapture
test result: ok. 39 passed; 0 failed; 0 ignored; 0 measured; 303 filtered out
```

## Limits

- This slice supports a second-turn transcript shape only.
- It does not automate a four-turn conversation loop.
- It does not extract assistant text from one streamed run and feed it into the
  next request; benchmark operators must still provide `--assistant-context`.
