# OpenAI streaming completion validation order

## Scope

This slice keeps request validation ahead of server availability for legacy
streaming completions.

After the unloaded-model availability fix, `POST /v1/completions` checked that
an engine was loaded before checking that streaming completions have exactly
one text prompt. That meant an invalid streaming request with multiple prompt
strings returned `503` on an unloaded server instead of the request-shape
`400`.

The handler now validates the streaming prompt shape before checking engine
availability or acquiring the inference permit.

## Evidence

Red:

```text
cargo test -p ferrite-server openai::availability_tests -- --nocapture
```

Failed with `streaming_completion_prompt_validation_runs_before_engine_availability`
returning `503` instead of expected `400`.

Green:

```text
cargo test -p ferrite-server openai::availability_tests -- --nocapture
```

Passed: 3 passed.

## Boundary

This only changes validation ordering for streaming legacy completions with
invalid prompt cardinality. Loaded streaming completions and unloaded valid
requests keep their existing behavior.
