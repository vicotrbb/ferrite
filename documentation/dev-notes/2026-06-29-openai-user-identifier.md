# OpenAI User Identifier

Date: 2026-06-29

## Slice

Ferrite's OpenAI-compatible chat and legacy completion endpoints now accept
`user` when it is a string. OpenAI documents this as an optional end-user
identifier used for monitoring and abuse detection, not as a text-generation
control. Ferrite treats it as local request metadata and does not pass it into
the inference core.

Source references:

- https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create
- https://developers.openai.com/api/reference/resources/completions/methods/create

## Implementation

- Added `crates/ferrite-server/src/openai/schema/user_identifier.rs` to keep
  user identifier validation separate from request structs.
- Updated chat and legacy completion unsupported-field detection to accept only
  missing `user` or string `user` values.
- Added fixture-backed route tests for chat and legacy completion requests with
  `user: "local-user-1"`.

## Red Test

```sh
cargo test -p ferrite-server user_identifier -- --nocapture
```

Failed before implementation with:

```text
unsupported completion field(s): user
unsupported chat completion field(s): user
```

## Validation

```sh
cargo test -p ferrite-server user_identifier -- --nocapture
cargo test -p ferrite-server openai::schema::user_identifier -- --nocapture
```

Both commands passed after implementation.

## Limits

This slice does not implement abuse monitoring, request attribution, audit
storage, hosted OpenAI safety behavior, or any generation behavior tied to the
`user` value. Non-string `user` values remain unsupported.
