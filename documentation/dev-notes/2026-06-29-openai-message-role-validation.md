# OpenAI Message Role Validation

Date: 2026-06-29

## Slice

Ferrite's OpenAI-compatible chat completions endpoint now parses unknown or
malformed message roles far enough to reject them through the normal unsupported
field path. Requests with unknown role strings or non-string role values now
return OpenAI-shaped `messages.role` errors instead of generic JSON body
deserialization failures.

OpenAI documents chat messages as a tagged set of role-specific objects such as
`developer`, `system`, `user`, `assistant`, `tool`, and deprecated `function`.
Ferrite accepts those supported local transcript roles and rejects all other
role shapes explicitly.

Source reference:

- https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create

## Implementation

- Replaced derived `ChatRole` deserialization with a small custom parser.
- Added an internal `Unknown` role marker for request validation.
- Added `messages.role` to chat-message unsupported-field reporting.
- Kept supported roles and prompt rendering labels unchanged.

## Red Tests

```sh
cargo test -p ferrite-server unknown_message_role -- --nocapture
cargo test -p ferrite-server non_string_message_role -- --nocapture
```

Before implementation, unknown role strings failed with an enum variant
deserialization error and non-string role values failed before Ferrite's
unsupported-field validation ran.

## Validation

```sh
cargo test -p ferrite-server unknown_message_role -- --nocapture
cargo test -p ferrite-server non_string_message_role -- --nocapture
cargo test -p ferrite-server openai::prompt -- --nocapture
```

All focused commands passed after implementation.

## Limits

This slice does not change the accepted local transcript roles, implement
additional hosted message semantics, or infer a default role when `role` is
omitted.
