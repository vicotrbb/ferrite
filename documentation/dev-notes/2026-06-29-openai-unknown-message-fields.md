# OpenAI Unknown Message Fields

Date: 2026-06-29

## Slice

Ferrite's OpenAI-compatible chat completion endpoint now rejects unknown
message-level fields instead of silently dropping them.

Before this slice, unknown keys inside `messages[]` were ignored by serde even
though unknown top-level chat request fields were already reported. That made a
request with per-message vendor metadata appear accepted while Ferrite only used
`role` and `content`.

The schema now keeps documented message metadata fields explicit:

- `name`
- `tool_call_id`

Those fields remain local no-op metadata in this slice. Truly unknown
message-level fields are reported as `messages.<field>`.

Reference:

- <https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create/>

## Red

The focused route test first sent a chat request with an unknown
`messages[].vendor_context` field:

```sh
cargo test -p ferrite-server unknown_message_fields -- --nocapture
```

It failed before implementation with `503` instead of the desired
validation-layer `400`, proving the request reached generation.

## Green

Changes:

- Added `name`, `tool_call_id`, and flattened extra-field capture to
  `ChatMessage`.
- Added prefixed nested-field reporting to the shared unsupported-field helper.
- Added a route test asserting `messages.vendor_context` appears in the
  OpenAI-shaped validation error.

Verification:

```sh
cargo test -p ferrite-server unknown_message_fields -- --nocapture
cargo test -p ferrite-server collects_prefixed_extra_unsupported_fields -- --nocapture
```

Both focused tests passed after implementation.

## Boundary

This slice does not implement tool-result replay semantics or render message
metadata into prompts. It only prevents unknown per-message protocol fields from
being silently accepted.
