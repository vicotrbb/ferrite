# ADR 0008: OpenAI-Compatible HTTP API

Date: 2026-06-28

Status: Accepted

## Context

Ferrite is a CPU-native inference runtime, not only a CLI benchmark tool. Users
should be able to point common local-LLM and OpenAI client workflows at Ferrite
with a custom base URL, similar to how people use local servers such as Ollama
or llama-server.

The research corpus already names an OpenAI-compatible API server as a product
milestone, but the documents were inconsistent about implementation shape:
some sections favored `axum` and `tokio`, while others suggested a custom
HTTP/JSON stack. The project goal already allows normal Rust crates for
generic infrastructure while keeping inference machinery Ferrite-owned.

The current OpenAI API reference keeps Chat Completions, legacy Completions,
and Models resources available. The compatibility target is therefore the
stable local-serving subset that mainstream clients expect:

- `GET /health`
- `GET /v1/models`
- `POST /v1/chat/completions`
- `POST /v1/completions`

## Decision

Ferrite will ship a focused HTTP server crate for OpenAI-compatible local
inference. The initial implementation should use standard Rust infrastructure
crates for HTTP, async I/O, JSON, CLI/config parsing, and logging. The server
must not wrap another inference runtime; it calls Ferrite-owned model loading,
tokenization, session, sampling, and generation code.

The first server slice is intentionally narrow:

- single local model loaded at startup;
- local bind address defaults to `127.0.0.1`;
- `GET /health` reports process readiness;
- `GET /v1/models` returns one OpenAI-shaped model entry for the loaded model;
- `POST /v1/chat/completions` accepts text chat messages and returns an
  OpenAI-shaped non-streaming response;
- `POST /v1/completions` accepts a text prompt and returns an OpenAI-shaped
  non-streaming response;
- unsupported OpenAI fields are either ignored only when harmless or rejected
  with an OpenAI-shaped error object when honoring them would be misleading;
- request execution is serialized or bounded until batching/concurrency has
  evidence-backed design;
- streaming is implemented as a follow-up slice using SSE chunks and
  `data: [DONE]`, not bolted onto the first response path.

The server code should stay modular. A future `crates/ferrite-server` crate
should use focused modules such as:

- `config` for bind address, model path, model id, context, token limits, and
  optional bearer-token policy;
- `state` for loaded-model/session ownership and bounded execution;
- `openai::schema` for request, response, chunk, and error JSON structs;
- `openai::routes` for endpoint handlers;
- `openai::prompt` for chat-message-to-prompt rendering;
- `openai::streaming` for SSE support once token streaming is available.

## Consequences

Ferrite's product surface is no longer CLI-only. Server compatibility becomes
a required milestone, and regressions should be tested with both direct HTTP
requests and at least one standard OpenAI client configured with Ferrite as
the base URL.

The server may depend on infrastructure crates because HTTP, JSON, SSE,
configuration, and logging are not Ferrite's inference differentiators. The
inference crates should remain independent of HTTP-specific types.

OpenAI compatibility does not mean full OpenAI API parity. The first supported
contract is local text generation. Tool calls, multimodal input, audio,
hosted-file APIs, fine-tuning APIs, remote auth administration, and the newer
Responses API are out of scope until explicit ADRs or plans add them.

## Alternatives Considered

- **Custom HTTP/JSON/SSE stack.** Rejected for the product server because it
  adds protocol risk and distracts from CPU inference work. It remains valid
  only as a constrained experiment if future evidence shows a real deployment
  need.
- **CLI-only runtime.** Rejected because it blocks common integration paths
  and makes Ferrite harder to use as a local model service.
- **Full OpenAI API clone.** Rejected because Ferrite needs a reliable local
  inference subset before it grows into broader API coverage.
- **Ollama-native API first.** Deferred. OpenAI compatibility covers the
  largest client ecosystem and also matches the common Ollama workflow of
  configuring OpenAI clients with a local base URL.

## Evidence

- `documentation/engineering/ferrite-goal-prompt.md` permits normal Rust
  crates for generic infrastructure while keeping inference machinery custom.
- `research/08-implementation-roadmap.md` already lists an OpenAI-compatible
  API phase with `/v1/chat/completions`, `/v1/completions`, `/v1/models`, and
  `/health`.
- OpenAI API reference, retrieved 2026-06-28:
  - Chat Completions create endpoint:
    <https://platform.openai.com/docs/api-reference/chat/create>
  - Completions create endpoint:
    <https://platform.openai.com/docs/api-reference/completions/create>
  - Models list endpoint:
    <https://platform.openai.com/docs/api-reference/models/list>
- `documentation/dev-notes/2026-06-29-openai-completion-seed.md` records
  focused compatibility evidence for accepting integer `seed` on legacy
  completions while preserving malformed-seed rejection.
- `documentation/dev-notes/2026-06-29-openai-chat-seed.md` records matching
  compatibility evidence for accepting integer `seed` on chat completions
  while preserving malformed-seed rejection.
- `documentation/dev-notes/2026-06-29-openai-generation-model-not-found.md`
  records compatibility evidence for returning `model_not_found` from
  generation endpoints when the requested model id is not loaded.
- `documentation/dev-notes/2026-06-29-openai-function-message-role.md`
  records compatibility evidence for accepting deprecated
  `role: "function"` chat transcript messages as local text context while
  keeping function calling unsupported.
- `documentation/dev-notes/2026-06-29-openai-function-message-name.md`
  records compatibility evidence for requiring `name` on deprecated function
  messages before treating them as local transcript text.
- `documentation/dev-notes/2026-06-29-openai-assistant-refusal-content.md`
  records compatibility evidence for accepting assistant `refusal` content
  parts as local transcript text while keeping hosted refusal semantics out of
  scope.
- `documentation/dev-notes/2026-06-29-openai-message-tool-call-fields.md`
  records compatibility evidence for rejecting active message-level
  tool/function call metadata instead of silently dropping it.
- `documentation/dev-notes/2026-06-29-openai-unknown-message-fields.md`
  records compatibility evidence for rejecting unknown message-level fields
  while preserving documented local no-op metadata fields.
- `documentation/dev-notes/2026-06-29-openai-message-metadata-types.md`
  records compatibility evidence for validating documented message metadata
  field types before treating them as local no-ops.
- `documentation/dev-notes/2026-06-29-openai-tool-message-id.md` records
  compatibility evidence for requiring `tool_call_id` on tool-role messages
  before treating them as local transcript text.
- `documentation/dev-notes/2026-06-29-openai-tool-call-id-role.md` records
  compatibility evidence for rejecting `tool_call_id` on non-tool messages.
- `documentation/dev-notes/2026-06-29-openai-real-http-rerun.md` records a
  fresh explicit rerun of real Tier 0 and Tier 1 OpenAI-compatible HTTP tests.
- `documentation/dev-notes/2026-06-29-openai-health-readiness.md` records
  compatibility evidence for deriving `/health` readiness from actual loaded
  model availability.
- `documentation/dev-notes/2026-06-29-openai-empty-logit-bias.md` records
  compatibility evidence for accepting empty `logit_bias` maps as local no-ops
  while keeping non-empty biasing unsupported.
- `documentation/dev-notes/2026-06-29-openai-assistant-audio-null.md` records
  compatibility evidence for accepting `audio: null` on assistant transcript
  messages while keeping non-null audio metadata unsupported.
- `documentation/dev-notes/2026-06-29-openai-assistant-refusal-null.md`
  records compatibility evidence for accepting `refusal: null` on assistant
  transcript messages while keeping non-null top-level refusal metadata
  unsupported.
- `documentation/dev-notes/2026-06-29-openai-parallel-tool-calls-without-tools.md`
  records compatibility evidence for accepting boolean `parallel_tool_calls`
  when no tools are configured while keeping active tool calling unsupported.
- `documentation/dev-notes/2026-06-29-openai-assistant-tool-call-content-optional.md`
  records compatibility evidence for parsing assistant tool-call transcript
  messages without `content` and rejecting the unsupported tool metadata
  explicitly.
- `documentation/dev-notes/2026-06-29-openai-refusal-content-role-boundary.md`
  records compatibility evidence for accepting refusal content parts only on
  assistant messages and rejecting them on user messages.
- `documentation/dev-notes/2026-06-29-openai-unsupported-content-parts.md`
  records compatibility evidence for returning explicit `messages.content`
  errors for unsupported multimodal content parts instead of JSON body
  deserialization errors.
- `documentation/dev-notes/2026-06-29-openai-malformed-content-parts.md`
  records compatibility evidence for returning explicit `messages.content`
  errors for malformed text content parts instead of JSON body deserialization
  errors.
- `documentation/dev-notes/2026-06-29-openai-token-prompts.md` records
  compatibility evidence for returning explicit `prompt` errors for documented
  legacy completion token prompt forms instead of JSON body deserialization
  errors.
- `documentation/dev-notes/2026-06-29-openai-message-role-validation.md`
  records compatibility evidence for returning explicit `messages.role` errors
  for unknown or malformed chat message roles instead of JSON body
  deserialization errors.
- `documentation/dev-notes/2026-06-29-openai-missing-message-role.md`
  records compatibility evidence for returning explicit `messages.role` errors
  when chat messages omit the required role field instead of JSON body
  deserialization errors.
- `documentation/dev-notes/2026-06-29-openai-missing-model.md` records
  compatibility evidence for returning explicit `model` errors when generation
  requests omit the required model field instead of JSON body deserialization
  errors.
- `documentation/dev-notes/2026-06-29-openai-missing-generation-inputs.md`
  records compatibility evidence for returning explicit `messages` and
  `prompt` errors when generation requests omit their required input fields
  instead of JSON body deserialization errors.
- `documentation/dev-notes/2026-06-29-openai-malformed-model.md` records
  compatibility evidence for returning explicit `model` errors when generation
  requests provide malformed model ids instead of JSON body deserialization
  errors.
- `documentation/dev-notes/2026-06-29-openai-malformed-messages.md` records
  compatibility evidence for returning explicit `messages` errors when chat
  requests provide malformed message arrays instead of JSON body
  deserialization errors.
