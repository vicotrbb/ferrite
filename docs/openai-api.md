# OpenAI API compatibility

Ferrite implements a focused local subset of the OpenAI API. Unknown or
non-neutral unsupported fields are rejected with structured errors instead of
being silently ignored.

## Endpoints

| Method | Path | Purpose |
| --- | --- | --- |
| `GET` | `/health` | Local readiness |
| `GET` | `/v1/models` | List the configured model |
| `GET` | `/v1/models/{model}` | Retrieve the configured model |
| `POST` | `/v1/completions` | Legacy text completion |
| `POST` | `/v1/chat/completions` | Chat completion |
| `OPTIONS` | supported `/v1/*` routes | CORS preflight |

Provider-style model IDs can contain slashes. Request model IDs must match the
configured `--model-id` exactly.

## Chat completions

Required fields are `model` and a non-empty `messages` array. Ferrite accepts
the `developer`, `system`, `user`, `assistant`, `tool`, and `function` role
names for text prompt rendering. Tool and function execution are not supported.
Requests that contain actual tool definitions, tool calls, audio, web search,
or non-text output requirements are rejected.

Supported generation controls are:

- `max_completion_tokens`, or the legacy `max_tokens`, but not malformed
  values or values above the server hard limit.
- `stream` as a Boolean.
- `stream_options.include_usage` as a Boolean.
- `stream_options.include_obfuscation` as a Boolean. It defaults to true for
  OpenAI-compatible streaming shape.
- `stop` as one non-empty string or at most four non-empty strings.
- `prompt_cache_key` as a non-empty string.
- `service_tier` in the locally accepted neutral form.
- `return_token_ids` as the Ferrite extension used by parity tests.
- `metadata.ferrite_cache_trace` with the string value `"true"` to enable cache
  trace observability.

Greedy decoding is the only sampling policy. Neutral compatibility values such
as `temperature` 0 or 1, `top_p` 1, `n` 1, zero penalties, empty tools, and
false logging options can be accepted. Values that request unsupported
behavior are rejected.

## Legacy completions

`prompt` accepts one string or an array of strings for non-streaming requests.
Streaming requires exactly one text prompt. `echo` can be true or false.
Supported limits, stop sequences, stream options, cache keys, and neutral
sampling values follow the same rules as chat completions.

## Streaming

Streaming responses use server-sent events and end with:

```text
data: [DONE]
```

When `stream_options.include_usage` is true, Ferrite sends a final usage chunk
before the terminal event. Client disconnects cancel request work through the
generation lifecycle and release admission permits.

## Errors

Ferrite returns OpenAI-shaped JSON errors for malformed JSON, invalid request
parameters, unsupported fields, authentication failure, missing models, token
limits, unavailable inference, rate limits, unknown routes, and disallowed
methods. Parameter errors identify the relevant field when possible.

Do not depend only on status text. Clients should use the HTTP status and the
structured error `type`, `message`, and `param` fields.

## Compatibility test coverage

The workspace includes in-memory HTTP tests and real-model tests through the
`async-openai` Rust client. Covered areas include models, completions, chat,
streaming, usage chunks, authentication, CORS, stop sequences, errors, queueing,
disconnects, caching, and response shape.
