# OpenAI Real Tier 1 HTTP Suite Serialization

Date: 2026-06-29

## Scope

This slice stabilizes the ignored real Tier 1 OpenAI HTTP integration suite in
`crates/ferrite-server/tests/openai_real_tier1_http.rs`.

The suite serves the local `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
artifact through Ferrite's OpenAI-compatible HTTP server and verifies:

- `POST /v1/completions`
- `POST /v1/completions` with `stream: true`
- `POST /v1/chat/completions`
- `POST /v1/chat/completions` with `stream: true`
- busy-server `429 Too Many Requests` behavior
- configured wait-for-inference behavior

## Failure

The full ignored suite failed when run with Rust's default test parallelism:

```sh
cargo test -p ferrite-server --test openai_real_tier1_http -- --ignored --nocapture
```

Observed result:

```text
test result: FAILED. 5 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 189.96s
```

The failing test was
`live_http_server_waits_for_concurrent_real_tier1_request`. Its second request
expected to wait for a permit, but received:

```text
HTTP/1.1 429 Too Many Requests
{"error":{"message":"inference request queue is full; retry later","type":"rate_limit_error","param":null,"code":null}}
```

## Root Cause

The isolated wait test passed:

```sh
cargo test -p ferrite-server --test openai_real_tier1_http live_http_server_waits_for_concurrent_real_tier1_request -- --ignored --nocapture
```

Observed result:

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 93.01s
```

This showed the server's configured wait path worked. The full suite failure was
self-contention: multiple ignored real-model HTTP tests loaded and served the
same CPU-heavy model concurrently, so the first streaming request in the wait
test could hold the single inference permit longer than the configured
180-second wait budget.

## Change

The test file now uses a file-local Tokio mutex and acquires it at the start of
each real-model test. This serializes expensive model-serving tests inside the
integration-test binary while preserving the intentional request concurrency
inside the two queue behavior tests.

No production server behavior changed.

## Verification

After the change:

```sh
cargo test -p ferrite-server --test openai_real_tier1_http -- --ignored --nocapture
```

Observed result:

```text
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 268.14s
```

## Limits

This proves the current Tier 1 Qwen2.5-0.5B Q4_K_M HTTP integration suite is
stable under its default invocation. It does not prove larger Tier 1 Qwen2.5
1.5B variants, throughput targets, memory budgets, or broad OpenAI API parity.
