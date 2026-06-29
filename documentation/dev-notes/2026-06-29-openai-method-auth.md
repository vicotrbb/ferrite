# OpenAI Method Auth

Date: 2026-06-29

## Scope

Require configured bearer authentication before returning OpenAI-shaped
method-not-allowed errors for known `/v1/*` routes.

This is an HTTP compatibility and local-server security slice only. It does
not change model loading, inference execution, request schemas, supported
methods, or the unauthenticated `/health` readiness path.

## Rationale

Ferrite's README documents that `/v1/*` endpoints require
`Authorization: Bearer <api-key>` when `--api-key` is configured. Known route
handlers enforced this for supported methods, and unknown `/v1/*` routes now
enforce it before not-found responses. The method-not-allowed fallback still
returned a `405` before checking auth.

Wrong-method requests to protected OpenAI routes should use the same auth
boundary. Authorized wrong-method requests still return the existing
OpenAI-shaped `method_not_allowed` error.

## Change

- Added a focused auth regression for unauthenticated `GET /v1/completions`
  when an API key is configured.
- Passed `ServerState` and headers into the method-not-allowed fallback.
- Applied the existing `ensure_authorized` gate before returning
  `method_not_allowed`.
- Left no-api-key method-not-allowed behavior unchanged.

## Verification

RED command:

```sh
cargo test -p ferrite-server openai::auth_tests -- --nocapture
```

Observed result before the method fallback auth change:

- `wrong_method_openai_routes_require_matching_bearer_token` failed:
  expected `401`, got `405`.

GREEN command:

```sh
cargo fmt --all
cargo test -p ferrite-server openai::auth_tests -- --nocapture
```

Observed result:

- `openai::auth_tests`: 6 passed, 0 failed.

Full slice gates:

- `cargo fmt --all -- --check`: passed.
- `git diff --check`: passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo test --workspace -- --nocapture`: passed.
- `ferrite-server` library tests: 229 passed, 0 failed.
- Real-model HTTP suites remained ignored because they require local GGUF
  model artifacts.

## Limits

This slice covers fixture HTTP method-fallback auth behavior only. It does not
rerun ignored real-model GGUF HTTP suites.
