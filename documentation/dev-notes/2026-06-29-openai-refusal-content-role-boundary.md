# OpenAI Refusal Content Role Boundary

Date: 2026-06-29

## Slice

Ferrite's OpenAI-compatible chat completion endpoint now retains whether a
message content array contains a `refusal` part so request validation can apply
the documented role boundary. Assistant refusal content parts remain accepted
as local transcript text. User messages with refusal content parts are rejected
as unsupported `messages.content` instead of being treated as user text.

OpenAI documents refusal content parts under assistant messages. User messages
support text inputs and multimodal input parts, but Ferrite's current local
text path accepts only text.

Source reference:

- https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create

## Implementation

- Added a `has_refusal_part` flag to `ChatContent`.
- Kept prompt rendering text-only and unchanged for supported content.
- Updated chat message validation to reject refusal content parts unless the
  message role is `assistant`.

## Red Test

```sh
cargo test -p ferrite-server user_refusal_content_parts -- --nocapture
```

Failed before implementation because the request passed unsupported-field
validation and reached model execution, returning service unavailable in the
fixture server state.

## Validation

```sh
cargo test -p ferrite-server user_refusal_content_parts -- --nocapture
cargo test -p ferrite-server assistant_refusal_content_parts -- --nocapture
cargo test -p ferrite-server chat_content -- --nocapture
```

All three commands passed after implementation.

## Limits

This slice does not add multimodal input support, hosted refusal semantics,
tool calling, or any change to local prompt rendering beyond stricter request
validation.
