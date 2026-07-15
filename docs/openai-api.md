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
| `POST` | `/v1/responses` | Bounded non-streaming text response |
| `OPTIONS` | supported `/v1/*` routes | CORS preflight |

Provider-style model IDs can contain slashes. Request model IDs must match the
configured `--model-id` exactly.

JSON request bodies are limited to 2 MiB before deserialization. Oversized,
malformed, and incorrectly typed bodies return an OpenAI-shaped
`invalid_request_error`. Authentication is checked before a protected body is
read or parsed.

`GET /health` always returns HTTP 200 with the configured model ID and actual
load readiness:

```json
{"model":"qwen2.5-0.5b-q4_k_m","ready":true}
```

A process without a loaded model reports `ready: false`. Generation endpoints
then return service unavailable rather than pretending the model is ready.

## Chat completions

Required fields are `model` and a non-empty `messages` array. Ferrite accepts
the `developer`, `system`, `user`, `assistant`, `tool`, and `function` role
names for text prompt rendering. Bounded function definitions, assistant tool
calls, and matching tool-result history are supported for compatible Qwen
ChatML templates. Ferrite reports calls to the client and never executes them.
Audio, web search, hosted tools, and non-text input are rejected.

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

### Sampling

Ferrite implements these OpenAI-shaped sampling fields:

| Field | Accepted values |
| --- | --- |
| `temperature` | number from 0 through 2 |
| `top_p` | number from 0 through 1 |
| `frequency_penalty` | number from -2 through 2 |
| `presence_penalty` | number from -2 through 2 |
| `logit_bias` | token-ID keys with number values from -100 through 100 |
| `seed` | signed 64-bit integer or `null` |

Ferrite also accepts `top_k`, `min_p`, and `repetition_penalty` as local
extensions. `top_k` is a non-negative integer, where zero disables the filter.
`min_p` is from 0 through 1, and `repetition_penalty` must be greater than zero.
Only `n: 1` is supported.

Omitting `temperature` preserves Ferrite's established exact-greedy default of
0. This is a Ferrite compatibility choice. Clients that require stochastic
sampling should send a positive temperature explicitly. With temperature 0 and
no active penalty or logit bias, inference keeps the fused argmax path and does
not materialize the complete logit vector. Penalties and logit bias can still
change deterministic selection at temperature 0, so they use full logits.
Probability filters apply when temperature is positive.

Every generation owns independent random state. Given the same Ferrite build,
model, prompt, parameters, and explicit seed, unrelated requests do not alter
the sampled token trace. Seeded output can still change when model bytes,
numeric kernels, or other generation inputs change.

Fused-greedy requests are eligible for the experimental continuous batch
scheduler for both streaming and non-streaming responses. Prefix-cache and
cache-trace options remain eligible. Requests that need sampling or logit
modification use the ordinary inference-permit path.

The field names and standard ranges follow the current
[Chat Completions](https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create)
and [legacy Completions](https://developers.openai.com/api/reference/resources/completions/methods/create)
references. Ferrite's omission defaults and local extensions are documented
above instead of being presented as hosted-service parity.

### Model chat templates

Ferrite reads `tokenizer.chat_template` from GGUF metadata and recognizes a
bounded subset for Qwen-style ChatML, Llama 3 headers, Llama 2 instruction
prompts, and Phi-3 turns. It does not execute arbitrary Jinja. Missing,
oversized, or unrecognized templates use an explicit role-labelled fallback
prompt.

Template metadata is capped at 64 KiB and rendered prompts are capped at 16
MiB. Tokenization preserves GGUF control and user-defined special tokens
atomically and honors configured BOS and EOS insertion without duplicating an
already rendered boundary token. Source-controlled fixtures cover the exact
Qwen2.5 template used by the reference artifact plus representative Llama 3,
Llama 2, and official Phi-3 templates.

Ferrite treats GGUF EOS, EOT, and EOM IDs plus the bounded native turn
terminators for those template families as end-of-generation. The terminal ID
counts toward completion usage but is not exposed as assistant content.

### Grammar-constrained JSON objects

Chat requests can select syntactic JSON-object mode with:

```json
{"response_format":{"type":"json_object"}}
```

At least one message must contain the word `JSON`, matching the upstream
safety convention. Ferrite constrains token selection before sampling, accepts
only prefixes that can still become one JSON object, and stops as soon as the
object is complete. Output is capped at 1 MiB and 64 nesting levels. Control
and special tokens are excluded from grammar candidates.

JSON-object mode is non-streaming and cannot be combined with stop sequences
or function tools. It guarantees valid JSON object syntax, not conformance to
an application schema. `json_schema` is rejected until schema constraints are
implemented. An exhausted token budget before a complete object is an explicit
generation error, not a partial JSON success.

### Function tool calls

Chat requests can provide up to 64 tools of type `function`. Function names
are at most 64 bytes and contain only ASCII letters, digits, `_`, or `-`.
Descriptions are capped at 4 KiB. Parameter schemas are JSON objects capped at
64 KiB, 32 levels, and 4,096 nodes. Duplicate names and unknown fields are
rejected. `tool_choice` accepts `none`, `auto`, `required`, or one named
function choice. `parallel_tool_calls` is enforced.

Tool prompting and output parsing currently require a Qwen-compatible ChatML
template. Tool generation is non-streaming and cannot use stop sequences or
JSON-object mode. Ferrite accepts at most 16 generated calls. Every call must
name a declared function and contain an arguments object no larger than 64 KiB.
The response uses the standard `message.tool_calls` shape and
`finish_reason: "tool_calls"`.

`strict: true` is accepted as tool metadata but does not yet turn the
parameters schema into a decoding grammar. Ferrite validates the returned call
envelope and JSON arguments, not application-specific schema constraints. The
caller remains responsible for validating arguments, authorizing the action,
executing the tool outside Ferrite, and returning a matching `tool_call_id`.
This boundary follows the
[OpenAI function-calling guide](https://developers.openai.com/api/docs/guides/function-calling)
without adding an arbitrary-code execution path to the server.

## Legacy completions

`prompt` accepts one string or an array of strings for non-streaming requests.
Streaming requires exactly one text prompt. `echo` can be true or false.
Supported limits, stop sequences, stream options, cache keys, and sampling
values follow the same rules as chat completions.

## Responses API

`POST /v1/responses` implements a local, non-streaming text subset of the
[Responses create method](https://developers.openai.com/api/reference/resources/responses/methods/create).
`input` is required and can be one non-whitespace string or an array of at most
256 text message objects. Message roles are `developer`, `system`, `user`, and
`assistant`. Content can be a string or at most 256 `input_text` parts, with
`output_text` parts accepted for prior assistant messages. Instructions and
input text together are capped at 1 MiB before template rendering.

Supported generation controls are `max_output_tokens` and the sampling fields
documented above. `instructions`, `metadata`, `user`, `safety_identifier`,
`service_tier`, and `prompt_cache_key` use the same local validation and cache
semantics as chat completions. Neutral local values such as `background: false`,
`store: false`, empty `include` and `tools` arrays, plain text output, no
reasoning, and disabled truncation are accepted.

The response contains a `response` object, one assistant `message` output with
an `output_text` content item, and Responses-shaped input, cached, output,
reasoning, and total token usage. A generation-length stop returns status
`incomplete` with reason `max_output_tokens`; a model or generation stop
returns `completed`.

Ferrite does not implement Responses streaming, background work, stored or
previous response state, conversations, tools, multimodal content, reasoning,
automatic truncation, or structured text formats. Non-neutral requests for
those features are rejected by field name. `store` is always false and no
response state leaves the process.

## Streaming

Streaming responses use server-sent events and end with:

```text
data: [DONE]
```

When `stream_options.include_usage` is true, Ferrite sends a final usage chunk
before the terminal event. Client disconnects cancel request work through the
generation lifecycle and release admission permits.

Each streaming worker writes one `openai_stream_lifecycle` line to standard
error. Cancelled streams report the observed disconnect stage,
`disconnect_observed_elapsed_ms`, and server-side `disconnect_to_finish_ms`.
Prompt-evaluation cancellation also reports the token and layer indexes where
the closed stream was observed. These fields contain timings and counters, not
prompt or generated text.

Generated chat and legacy completion chunks include a Ferrite `token_ids`
extension for exact-token parity checks. Ferrite omits the field when a request
uses stop sequences because filtered text no longer has a one-to-one token
mapping. Echoed legacy-completion prompt chunks also omit it.

## Errors

Ferrite returns OpenAI-shaped JSON errors for malformed JSON, invalid request
parameters, unsupported fields, authentication failure, missing models, token
limits, unavailable inference, rate limits, unknown routes, and disallowed
methods. Parameter errors identify the relevant field when possible.

Do not depend only on status text. Clients should use the HTTP status and the
structured error `type`, `message`, and `param` fields.

| Status | Typical meaning |
| ---: | --- |
| 400 | malformed JSON, invalid parameter, token limit, or unsupported option |
| 401 | missing or incorrect bearer token |
| 404 | unknown route or requested model ID |
| 405 | method is not allowed for a known route |
| 429 | inference admission timed out or was unavailable immediately |
| 500 | unexpected internal generation failure |
| 501 | recognized operation is not implemented |
| 503 | no model is loaded or inference is otherwise unavailable |

## Compatibility test coverage

The workspace includes in-memory HTTP tests and real-model tests through the
`async-openai` Rust client. Covered areas include models, completions, chat,
Responses text requests, JSON-object grammar, function-call parsing, streaming,
usage chunks, authentication, CORS, stop sequences, errors, queueing,
disconnects, caching, and response shape.
