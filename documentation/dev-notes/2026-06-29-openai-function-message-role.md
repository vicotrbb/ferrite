# OpenAI Function Message Role

## Slice

Ferrite's OpenAI-compatible chat endpoint now accepts the deprecated
`role: "function"` chat message form as plain text transcript context.

OpenAI still documents `ChatCompletionFunctionMessageParam` for Chat
Completions. Ferrite does not implement function calling in this slice. It only
accepts already-materialized function transcript text and renders it into the
local prompt as:

```text
function: <content>
```

Tool calls, non-empty `functions`, and active `function_call` requests remain
unsupported.

Reference:

- <https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create/>

## Red

The route test first required a chat completion request with a deprecated
function message role to return the normal fixture response:

```sh
cargo test -p ferrite-server chat_endpoint_accepts_deprecated_function_message_role -- --nocapture
```

The first failure proved the role was rejected during request deserialization:

```text
Failed to deserialize the JSON body into the target type
```

After adding the role enum variant, the same test exposed a fixture-only issue:
the tiny chat tokenizer fixture did not contain `function: ` and therefore
could not encode the rendered prompt.

## Green

Changes:

- Added `ChatRole::Function`.
- Rendered `ChatRole::Function` as the transcript label `function`.
- Extended the small chat GGUF fixture vocabulary with `function: ` and neutral
  fixture tensor rows.

Verification:

```sh
cargo test -p ferrite-server chat_endpoint_accepts_deprecated_function_message_role -- --nocapture
```

Result:

```text
test openai::routes_tests::chat_endpoint_accepts_deprecated_function_message_role ... ok
```

## Boundary

This is compatibility for transcript replay only. It does not claim support for
OpenAI function calling, tool execution, hosted tools, or structured tool-call
generation. Those remain explicit future scope under ADR 0008.
