# OpenAI Assistant Refusal Content Parts

## Slice

Ferrite's OpenAI-compatible chat endpoint now accepts assistant transcript
content parts with `type: "refusal"` and treats the `refusal` text as local
prompt context.

OpenAI documents assistant message content as either plain text or an array of
text/refusal content parts. Ferrite does not implement hosted refusal behavior,
safety policy, or refusal generation in this slice. It only preserves
already-materialized assistant transcript text in the local prompt.

Reference:

- <https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create/>

## Red

The route test first required a chat completion request containing an assistant
message with refusal content parts to return the normal fixture response:

```sh
cargo test -p ferrite-server chat_endpoint_accepts_assistant_refusal_content_parts -- --nocapture
```

The expected failure was request-body deserialization:

```text
Failed to deserialize the JSON body into the target type
```

## Green

Changes:

- Replaced the text-only content-part parser with a small tagged enum for
  `type: "text"` and `type: "refusal"`.
- Kept multimodal parts rejected by the existing unsupported content test.
- Added parser-level coverage for refusal parts and route-level coverage for
  chat completions.

Verification:

```sh
cargo test -p ferrite-server chat_endpoint_accepts_assistant_refusal_content_parts -- --nocapture
cargo test -p ferrite-server chat_content -- --nocapture
```

Result:

```text
test openai::routes_tests::chat_endpoint_accepts_assistant_refusal_content_parts ... ok
test openai::schema::chat_content::tests::deserializes_refusal_content_parts ... ok
```

## Boundary

This compatibility slice does not claim support for OpenAI safety semantics,
policy enforcement, hosted refusal generation, or multimodal assistant content.
It only accepts a documented text-bearing transcript shape and keeps unsupported
non-text content parts rejected.
