# OpenAI HTTP Throughput Client

Date: 2026-06-29

## Scope

This slice adds a small release-oriented benchmark client binary:

```text
ferrite-openai-throughput
```

The client sends OpenAI-compatible legacy completion requests to a separately
running Ferrite server and prints:

- `openai_http_completion_requests`
- `elapsed_ms`
- `requests_per_second`

This is benchmark infrastructure only. It does not claim release throughput
until the server and client are both run with explicit release commands and the
result is recorded under `documentation/benchmarks/`.

## Test-Driven Evidence

Red:

```text
cargo test -p ferrite-server throughput_client -- --nocapture
error[E0433]: cannot find type `ThroughputClientConfig` in this scope
error[E0425]: cannot find function `completion_request_body` in this scope
```

First green attempt exposed unstable JSON field order:

```text
assertion `left == right` failed
left: "{\"max_tokens\":2,\"model\":\"fixture-model\",\"prompt\":\"measure this\"}"
right: "{\"model\":\"fixture-model\",\"prompt\":\"measure this\",\"max_tokens\":2}"
```

Green:

```text
cargo test -p ferrite-server throughput_client -- --nocapture
test throughput_client::tests::parses_minimal_completion_benchmark_config ... ok
test throughput_client::tests::builds_openai_compatible_completion_request_body ... ok
```

## What Changed

- Added `crates/ferrite-server/src/throughput_client.rs`.
- Added `crates/ferrite-server/src/bin/ferrite-openai-throughput.rs`.
- Documented release server/client usage in `README.md`.

## Remaining Work

The next benchmark slice should run `target/release/ferrite-server` and
`target/release/ferrite-openai-throughput` against Qwen2.5-1.5B Q8_0 with an
explicit host profile and record the measured result under
`documentation/benchmarks/`.
