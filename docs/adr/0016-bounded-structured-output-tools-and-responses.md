# ADR 0016: Bounded structured output, tools, and Responses

Date: 2026-07-13

Status: Accepted

## Context

Ferrite already validated OpenAI-shaped chat and completion requests, but
application integration still required three reliability boundaries:

1. syntactically valid machine-readable output;
2. function-call interchange without arbitrary execution inside the server;
3. a small Responses API compatibility path that preserves local-only state.

Accepting fields and ignoring them would create false compatibility. Parsing
unbounded JSON or model-emitted tool envelopes would create hostile-input and
memory risks. Post-processing invalid free-form output would not make the model
generation itself constrained.

## Decision

Chat `response_format: {"type":"json_object"}` uses a token-level prefix
grammar. The grammar filters candidates before sampling, accepts only UTF-8
prefixes that can become one JSON object, and stops when the object is complete.
It caps output at 1 MiB and nesting at 64. JSON mode is non-streaming and cannot
be combined with stop sequences or tools. JSON Schema mode remains rejected.

Chat function tools use bounded definitions and the exact Qwen-compatible
ChatML tool prompt shape. Tool names, descriptions, parameter JSON, nesting,
node count, definition count, generated argument size, and parsed-call count
all have explicit limits. Tool choice and parallel-call policy are enforced.
Generated `<tool_call>` envelopes must name declared functions and contain JSON
object arguments. Responses use the OpenAI chat `tool_calls` shape. Ferrite
never authorizes, invokes, imports, shells out to, or otherwise executes a
function call.

`POST /v1/responses` supports non-streaming local text only. Input is one string
or a bounded array of developer, system, user, and assistant text messages.
Instructions, token limits, sampling, metadata, safety identifiers, service
tier, and prompt-cache namespaces map onto existing validated runtime paths.
The output uses a `response` object, message and `output_text` items, incomplete
status for token-length exhaustion, and Responses-shaped usage.

Hosted state, storage, background work, conversations, previous response IDs,
Responses tools, multimodal input, reasoning, automatic truncation, and
Responses structured formats are rejected unless their value is explicitly
neutral. `store` is always false.

## Consequences

Applications can request syntactically valid JSON objects and can perform a
standard external tool loop. The application still validates business schemas,
authorizes actions, executes tools, and supplies matching result history.

The Responses endpoint is useful to text clients without pretending to be a
hosted conversation service. It shares authentication, CORS, backpressure,
sampling, batching, cancellation, and prefix-cache accounting with the existing
server.

`strict: true` on a function definition is retained as metadata but does not
compile its JSON Schema into the decoder grammar. Returned arguments are valid
bounded JSON objects, not guaranteed schema instances. This limitation is
documented and tested as an explicit boundary.

## Alternatives Considered

- Repair free-form JSON after generation. Rejected because it can change model
  output and cannot guarantee a valid result within the token budget.
- Execute tool functions inside Ferrite. Rejected because local inference does
  not grant application authority, credentials, or an arbitrary-code boundary.
- Accept every Responses field as a no-op. Rejected because clients would rely
  on storage, reasoning, tools, or truncation semantics that do not exist.
- Implement streaming structured output immediately. Rejected until partial
  grammar state and chunk semantics have a dedicated compatibility gate.
- Treat `strict` as schema conformance without constrained decoding. Rejected
  because envelope validation is not JSON Schema validation.

## Evidence

- `crates/ferrite-server/src/runtime/json_grammar.rs` implements the bounded
  prefix grammar and adversarial parser tests.
- `crates/ferrite-server/src/openai/schema/tool_options.rs` owns definition and
  call parsing limits.
- `crates/ferrite-server/src/openai/schema/responses.rs` owns the Responses
  request and response subset.
- Route tests cover generation, cache usage, authentication, CORS, malformed
  input, unsupported fields, and response shapes.
- The official
  [structured output guide](https://developers.openai.com/api/docs/guides/structured-outputs),
  [function-calling guide](https://developers.openai.com/api/docs/guides/function-calling),
  and [Responses create reference](https://developers.openai.com/api/reference/resources/responses/methods/create)
  define the upstream shapes that Ferrite narrows explicitly.
