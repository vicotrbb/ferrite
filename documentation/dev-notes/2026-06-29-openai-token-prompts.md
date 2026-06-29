# OpenAI Token Prompts

Date: 2026-06-29

## Slice

Ferrite's OpenAI-compatible legacy completions endpoint now parses documented
token prompt forms far enough to reject them through the normal unsupported
field path. Requests with `prompt` as an array of token ids or an array of token
id arrays now return OpenAI-shaped `prompt` errors instead of generic JSON body
deserialization failures.

OpenAI documents legacy completion `prompt` as a string, an array of strings,
an array of token ids, or an array of token-id arrays. Ferrite's local server
currently supports text prompts only, so token prompt forms are explicit future
scope rather than silently interpreted.

Source reference:

- https://developers.openai.com/api/reference/resources/completions/methods/create

## Implementation

- Extended `CompletionPrompt` with a narrow unsupported-form marker.
- Preserved supported string and string-array prompt parsing.
- Recognized token-array and token-array-batch forms as unsupported local prompt
  shapes.
- Added `prompt` to completion request unsupported-field validation when token
  prompt forms are received.

## Red Tests

```sh
cargo test -p ferrite-server token_prompt_forms -- --nocapture
cargo test -p ferrite-server token_prompt_array -- --nocapture
cargo test -p ferrite-server token_prompt_array_batch -- --nocapture
```

Before implementation, the parser test failed because `[1,2,3]` did not match
any supported prompt wire shape, and both route tests failed with body
deserialization errors.

## Validation

```sh
cargo test -p ferrite-server token_prompt_forms -- --nocapture
cargo test -p ferrite-server token_prompt_array -- --nocapture
cargo test -p ferrite-server token_prompt_array_batch -- --nocapture
```

All three focused commands passed after implementation.

## Limits

This slice does not implement token-id prompt execution. It also does not add
tokenizer-specific validation for whether incoming token ids are in range for
the loaded local model.
