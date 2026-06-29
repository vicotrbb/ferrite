# OpenAI HTTP Fixture Throughput Harness

Date: 2026-06-29

## Scope

This note tracks the fixture-level throughput harness for Ferrite's
OpenAI-compatible legacy-completion HTTP path. The harness uses Ferrite's
existing live fixture server and direct HTTP helper, validates every response
as a successful OpenAI-shaped text completion, and records elapsed time plus
derived requests per second.

The goal is measurement infrastructure only. This does not prove Tier 1 server
throughput, multi-client throughput, batching behavior, concurrent successful
serving, long-running steady-state behavior, or real-model request rate.

## Test-Driven Evidence

Red:

```text
cargo test -p ferrite-server --test openai_http_throughput -- --nocapture
error[E0425]: cannot find function `measure_sequential_completion_requests` in this scope
```

Green:

```text
cargo test -p ferrite-server --test openai_http_throughput -- --nocapture
test live_http_server_measures_sequential_completion_request_rate ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## What Changed

- Added `crates/ferrite-server/tests/openai_http_throughput.rs`.
- Added `crates/ferrite-server/tests/support/throughput.rs` so sequential and
  queued batch measurement helpers stay out of the endpoint regression tests.
- Added a fixture-server configuration hook for tests that need bounded
  inference waiting.
- Validated each measured request for HTTP `200`, `text_completion` object
  shape, fixture model ID, and deterministic `winner` completion text.
- Covered both sequential request batches and a queued concurrent batch using
  Ferrite's single-inference-permit server with a bounded wait window.

Follow-up red:

```text
cargo test -p ferrite-server --test openai_http_throughput -- --nocapture
error[E0432]: unresolved import `support::throughput`
error[E0599]: no associated function or constant named `start_configured`
```

Follow-up green:

```text
cargo test -p ferrite-server --test openai_http_throughput -- --nocapture
test live_http_server_measures_queued_concurrent_completion_request_rate ... ok
test live_http_server_measures_sequential_completion_request_rate ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Remaining Work

Real OpenAI-compatible server throughput still needs a separate bounded
benchmark protocol with explicit model, prompt, token count, request shape,
client count, and host limits. That future result should be recorded under
`documentation/benchmarks/` and should not reuse this fixture harness as
evidence for real Tier 1 throughput.
