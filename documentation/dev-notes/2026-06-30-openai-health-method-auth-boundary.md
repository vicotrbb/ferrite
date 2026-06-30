# OpenAI health method auth boundary

## Context

Ferrite documents `/health` as unauthenticated even when `--api-key` protects
`/v1/*` OpenAI-compatible routes. The existing method-not-allowed fallback
checked bearer auth for every wrong-method request, which meant
`POST /health` returned `401 authentication_error` instead of a normal method
error.

## Slice

Keep wrong-method `/v1/*` requests protected by bearer auth, but leave
wrong-method `/health` unauthenticated.

## TDD

RED:

- `cargo test -p ferrite-server --lib wrong_method_health_route_does_not_require_bearer_token -- --nocapture`

Observed failure:

- expected `405`, got `401`.

GREEN:

- Threaded `OriginalUri` into the method-not-allowed fallback.
- Applied bearer-auth enforcement only when the wrong-method path starts with
  `/v1/`, matching the existing unknown-route fallback.

## Validation

Executed:

- `cargo test -p ferrite-server --lib openai::auth_tests -- --nocapture`

Result:

- 8 passed, 0 failed.

This preserves authenticated wrong-method behavior for protected
OpenAI-compatible `/v1/*` routes while keeping `/health` open for local
readiness checks. It does not change the supported method set or add new
endpoints.
