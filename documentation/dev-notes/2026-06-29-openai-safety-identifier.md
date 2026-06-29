# OpenAI Safety Identifier

Date: 2026-06-29

## Slice

Ferrite's OpenAI-compatible chat endpoint now accepts `safety_identifier` when
it is a string of at most 64 characters. The official Chat Completions API
documents this as an optional stable identifier for detecting application users
who may violate usage policies. Ferrite treats it as local request metadata and
does not pass it into the inference core.

Source reference:

- https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create

## Implementation

- Added `crates/ferrite-server/src/openai/schema/safety_identifier.rs` to keep
  identifier validation separate from chat request logic.
- Updated chat unsupported-field detection to accept missing
  `safety_identifier` or string `safety_identifier` values up to 64 characters.
- Added a fixture-backed route test for a request with `safety_identifier`.
- Added route-level rejection regressions for non-string and overlength
  `safety_identifier`.

## Red Test

```sh
cargo test -p ferrite-server chat_endpoint_accepts_safety_identifier -- --nocapture
```

Failed before implementation with:

```text
unsupported chat completion field(s): safety_identifier
```

## Validation

```sh
cargo test -p ferrite-server chat_endpoint_accepts_safety_identifier -- --nocapture
cargo test -p ferrite-server openai::schema::safety_identifier -- --nocapture
cargo test -p ferrite-server safety_identifier -- --nocapture
```

All commands passed after implementation.

## Limits

This slice does not implement hosted OpenAI abuse monitoring, user policy
checks, request attribution storage, audit search, or generation behavior tied
to `safety_identifier`.
