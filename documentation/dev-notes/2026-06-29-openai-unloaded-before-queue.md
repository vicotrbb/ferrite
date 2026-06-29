# OpenAI unloaded-model availability before queue state

## Scope

This slice makes unloaded-model generation errors deterministic for the
OpenAI-compatible chat and legacy completions endpoints.

Before this change, a request for the configured model could return
`429 rate_limit_error` if the inference permit was already held, even when no
model engine was loaded. That made queue state visible before the more useful
server availability state. The route handlers now verify that an inference
engine exists before acquiring the inference permit.

## Evidence

Red:

```text
cargo test -p ferrite-server openai::availability_tests -- --nocapture
```

Failed with both new tests returning `429` instead of expected `503`.

Green:

```text
cargo test -p ferrite-server openai::availability_tests -- --nocapture
```

Passed: 2 passed.

## Boundary

This does not change loaded-model queue behavior. Requests that have a loaded
engine and cannot acquire the inference permit still return `429`.
