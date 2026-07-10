# ADR 0008: OpenAI-compatible HTTP API

Date: 2026-06-28

Status: Accepted

## Context

Ferrite must be usable as a local model service, not only as a benchmark CLI.
Common clients already understand an OpenAI-style base URL, while implementing
custom HTTP, JSON, and SSE infrastructure would distract from the inference
engine and add protocol risk.

## Decision

`ferrite-server` uses established Rust infrastructure for HTTP, async I/O, and
JSON while calling Ferrite-owned model, tokenizer, session, and generation
code. HTTP types do not enter the model or inference crates.

The maintained API contains:

- `GET /health`;
- `GET /v1/models`;
- `GET /v1/models/{model}`;
- `POST /v1/completions`;
- `POST /v1/chat/completions`;
- CORS preflight for supported routes;
- JSON and SSE responses, including `data: [DONE]`;
- optional bearer authentication, token limits, bounded admission, and
  structured OpenAI-shaped errors.

Ferrite implements a focused text-generation subset. Neutral compatibility
values can be accepted, but a field that requests unsupported behavior is
rejected instead of silently ignored. Authentication is checked before parsing
protected request bodies. Provider-style model IDs are matched exactly.

The default bind address is localhost. TLS, internet-facing rate limiting,
durable audit logs, and process isolation belong to the deployment boundary.

## Consequences

API behavior is a tested product contract. Schema, error, streaming,
cancellation, CORS, queue, cache, and third-party client changes require HTTP
tests in addition to inference tests.

OpenAI compatibility does not imply complete API parity. Tool execution,
multimodal generation, audio, hosted files, fine tuning, embeddings, and the
Responses API remain unsupported until explicitly designed and tested.

## Alternatives considered

- **Custom HTTP and JSON implementation.** Rejected because mature crates
  handle generic protocol infrastructure more safely.
- **CLI-only operation.** Rejected because it blocks common local client and
  service integrations.
- **Full API clone.** Rejected because explicit, reliable subset behavior is
  preferable to broad silent incompatibility.

## Evidence

- [`../openai-api.md`](../openai-api.md) is the maintained compatibility table.
- `crates/ferrite-server/src/openai/` contains schemas, validation, routes, and
  streaming lifecycle code.
- `crates/ferrite-server/tests/openai_client_*.rs` and `openai_http.rs` cover
  direct HTTP and `async-openai` client behavior.
- [`../safety.md`](../safety.md) defines the server input and deployment
  boundary.
