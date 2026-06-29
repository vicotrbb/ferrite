# OpenAI Missing Message Role

Date: 2026-06-29

## Slice

Ferrite's OpenAI-compatible chat completions endpoint now parses chat messages
that omit `role` far enough to reject them through the normal unsupported-field
path. Requests with a missing message role now return OpenAI-shaped
`messages.role` errors instead of generic JSON body deserialization failures.

OpenAI documents chat messages as role-specific objects, with role literals
such as `developer`, `system`, `user`, `assistant`, `tool`, and deprecated
`function`. Ferrite accepts those supported local transcript roles and rejects
missing roles explicitly.

Source reference:

- https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create

## Implementation

- Derived `Default` for `ChatRole` with the internal `Unknown` marker as the
  omitted-role fallback.
- Added `#[serde(default)]` on `ChatMessage::role`.
- Reused the existing `messages.role` unsupported-field validation path.

## Red Tests

```sh
cargo test -p ferrite-server missing_message_role -- --nocapture
```

Before implementation, the schema test failed with `missing field 'role'`
before Ferrite's unsupported-field validation could run.

## Validation

```sh
cargo test -p ferrite-server missing_message_role -- --nocapture
cargo test -p ferrite-server message_without_role -- --nocapture
```

Both focused commands passed after implementation.

## Limits

This slice does not infer or default to a supported OpenAI role. Missing roles
remain invalid local chat input and are rejected before prompt rendering.
