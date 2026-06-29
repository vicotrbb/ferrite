# OpenAI CORS

Date: 2026-06-29

## Scope

Add minimal CORS support for Ferrite's local OpenAI-compatible HTTP surface.

This is an HTTP compatibility slice for local browser and webview clients. It
does not change model loading, inference execution, request schemas, bearer
authentication for normal protected requests, streaming frame shape, or the
open `/health` readiness endpoint.

## Rationale

OpenAI-compatible browser clients can send `OPTIONS` preflight requests before
calling local `POST /v1/chat/completions` or `POST /v1/completions`.

When `--api-key` is configured, Ferrite correctly requires
`Authorization: Bearer <api-key>` for normal `/v1/*` requests. Browser
preflight requests cannot carry the eventual bearer token, so the server needs
to answer preflight without entering the normal auth gate. Actual `/v1/*`
responses also need an `Access-Control-Allow-Origin` header or the browser can
still block the result.

## Change

- Added an auth/protocol regression for unauthenticated
  `OPTIONS /v1/chat/completions` with CORS preflight headers.
- Added an auth/protocol regression that an authorized `GET /v1/models`
  response includes `Access-Control-Allow-Origin`.
- Added explicit `OPTIONS` handlers for supported OpenAI-compatible routes.
- Added a small route middleware that attaches the same CORS headers to normal
  `/v1/*` responses.
- Documented the local CORS behavior in the README.

## Verification

RED command for preflight:

```sh
cargo test -p ferrite-server openai::auth_tests -- --nocapture
```

Observed failure before the preflight handler:

- `openai_cors_preflight_does_not_require_bearer_token` failed:
  expected `204`, got `401`.

RED command for actual response headers:

```sh
cargo test -p ferrite-server openai::auth_tests -- --nocapture
```

Observed failure before the response-header middleware:

- `protected_openai_routes_include_cors_response_header` failed because
  `access-control-allow-origin` was missing.

GREEN command:

```sh
cargo test -p ferrite-server openai::auth_tests -- --nocapture
```

Observed result:

- `openai::auth_tests`: 8 passed, 0 failed.

Regression caught during full gates:

- An initial `/v1/*` wildcard preflight route changed unknown non-`OPTIONS`
  OpenAI paths from the expected `404` to `405`.
- `cargo test --workspace -- --nocapture` caught this through
  `openai::request_error_tests::unknown_openai_route_returns_openai_error_body`.
- The wildcard route was removed; preflight support is now explicit for
  supported endpoints, preserving unknown-route `404` behavior.

Final slice gates:

- `cargo fmt --all -- --check`: passed.
- `git diff --check`: passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo test -p ferrite-server openai::auth_tests -- --nocapture`: passed.
- `cargo test -p ferrite-server openai::request_error_tests -- --nocapture`:
  passed.
- `cargo test --workspace -- --nocapture`: passed.
- `ferrite-server` library tests: 231 passed, 0 failed.
- Ignored real-model GGUF HTTP suites remained ignored by the default
  workspace run.

## Limits

This slice intentionally uses permissive local CORS headers for Ferrite's
OpenAI-compatible `/v1/*` responses and explicit preflight on supported local
OpenAI-compatible endpoints. It does not add configurable origin allowlists,
credentials support, hosted OpenAI parity, unknown-route preflight aliases, or
real GGUF model reruns.
