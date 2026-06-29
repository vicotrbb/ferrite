# OpenAI Unknown Route Auth

Date: 2026-06-29

## Scope

Require configured bearer authentication before returning OpenAI-shaped
not-found errors for unknown `/v1/*` routes.

This is an HTTP compatibility and local-server security slice only. It does
not change model loading, inference execution, known route handlers, request
schemas, or the unauthenticated `/health` readiness path.

## Rationale

Ferrite's README documents that `/v1/*` endpoints require
`Authorization: Bearer <api-key>` when `--api-key` is configured. Known OpenAI
routes already enforced this, but the fallback for unknown OpenAI paths such as
`/v1/responses` returned a route-not-found error before checking auth.

Unsupported OpenAI routes should still return OpenAI-shaped not-found errors,
but only after satisfying the same `/v1/*` auth boundary.

## Change

- Added a focused auth regression for unauthenticated `GET /v1/responses`.
- Passed `ServerState` and headers into the fallback handler.
- Applied the existing `ensure_authorized` gate before returning
  `route_not_found` for paths under `/v1/`.
- Left non-`/v1/` fallback behavior unchanged.

## Verification

RED command:

```sh
cargo test -p ferrite-server openai::auth_tests -- --nocapture
```

Observed result before the fallback auth change:

- `unknown_openai_routes_require_matching_bearer_token` failed:
  expected `401`, got `404`.

GREEN command:

```sh
cargo fmt --all
cargo test -p ferrite-server openai::auth_tests -- --nocapture
```

Observed result:

- `openai::auth_tests`: 5 passed, 0 failed.

Final slice gates:

```sh
cargo fmt --all -- --check
git diff --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace -- --nocapture
```

Observed results:

- Formatting check passed.
- Whitespace check passed.
- Workspace clippy passed with `-D warnings`.
- Workspace tests passed; `ferrite-server` reported 228 library tests passed,
  including the unknown OpenAI route auth regression.
- Ignored real-model GGUF HTTP suites remained ignored by the default workspace
  test command.

## Limits

This slice covers fixture HTTP fallback auth behavior only. It does not rerun
ignored real-model GGUF HTTP suites.
