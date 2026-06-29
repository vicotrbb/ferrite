# OpenAI Function Message Name

Date: 2026-06-29

## Slice

Ferrite's OpenAI-compatible chat completion endpoint now rejects deprecated
`role: "function"` messages that omit `name`.

OpenAI documents `ChatCompletionFunctionMessageParam` as `{ content, name,
role }`. Ferrite still treats accepted function messages as local transcript
text, but accepting an unnamed function message would make malformed function
transcript data look supported.

Reference:

- <https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create/>

## Red

The focused route test first sent a function message without `name`:

```sh
cargo test -p ferrite-server function_message_without_name -- --nocapture
```

It failed before implementation with `503` instead of the desired
validation-layer `400`, proving the request reached generation.

## Green

Changes:

- Added role-aware `ChatMessage` validation for `name`.
- `name` remains optional string metadata for non-function message roles.
- Function messages now require a string `name`.
- Missing or malformed names are reported as `messages.name`.

Verification:

```sh
cargo test -p ferrite-server function_message_without_name -- --nocapture
cargo test -p ferrite-server malformed_message_name -- --nocapture
cargo test -p ferrite-server chat_endpoint_accepts_deprecated_function_message_role -- --nocapture
```

All focused tests passed after implementation.

## Boundary

This slice does not implement function calling or tool execution. It only
keeps the deprecated function-message transcript form aligned with the
documented role-specific required field before Ferrite renders it as local text
context.
