# OpenAI Health Readiness

Date: 2026-06-29

## Slice

Ferrite's unauthenticated `GET /health` endpoint now reports readiness from
the actual loaded-model state.

Previously, `/health` returned `ready: true` for a configured model id even
when no inference engine was loaded. That contradicted the model catalog
boundary, where `/v1/models` already returns an empty list until a model is
available.

## Red

The focused route test first expected a server without a loaded engine to
return `ready: false`:

```sh
cargo test -p ferrite-server health_endpoint_reports_not_ready_without_loaded_model -- --nocapture
```

It failed before implementation because the health route returned
`ready: true`.

## Green

Changes:

- `HealthResponse` now accepts an explicit readiness value.
- The health route derives readiness from `ServerState::has_loaded_model()`.
- Added a fixture-backed health test proving `ready: true` when an engine is
  loaded.

Verification:

```sh
cargo test -p ferrite-server health_endpoint_reports -- --nocapture
```

Both focused health tests passed after implementation.

## Boundary

`/health` remains unauthenticated and returns `200 OK` for local process
probes. The readiness boolean indicates whether Ferrite has a model loaded and
can advertise local inference availability; it does not prove model-output
quality, throughput, or real GGUF artifact coverage.
