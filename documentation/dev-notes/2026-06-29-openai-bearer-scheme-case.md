# OpenAI Bearer Scheme Case

Date: 2026-06-29

## Scope

Accept `Authorization` bearer schemes case-insensitively on protected
OpenAI-compatible local server routes.

This is an HTTP compatibility slice only. It does not change API-key storage,
model loading, inference execution, request schemas, or the unauthenticated
`/health` readiness path.

## Rationale

Ferrite previously compared the entire authorization header to
`Bearer <api-key>`. That kept the token exact, but also made the auth scheme
case-sensitive. For a local OpenAI-compatible HTTP server, accepting
`bearer <api-key>` improves compatibility at the protocol boundary without
weakening token comparison.

## Change

- Added a focused auth regression test for `authorization: bearer local-secret`.
- Replaced the exact header-string comparison with route-local bearer parsing.
- Kept the token comparison exact.
- Kept missing, malformed, and wrong-token authorization failures on the
  existing OpenAI-shaped `authentication_error` path.

## Verification

RED command:

```sh
cargo test -p ferrite-server openai::auth_tests -- --nocapture
```

Observed result before the route change:

- `protected_openai_routes_accept_case_insensitive_bearer_scheme` failed:
  expected `200`, got `401`.

GREEN command:

```sh
cargo fmt --all
cargo test -p ferrite-server openai::auth_tests -- --nocapture
```

Observed result:

- `openai::auth_tests`: 3 passed, 0 failed.

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
- Workspace tests passed; `ferrite-server` reported 226 library tests passed,
  including the new bearer-scheme regression.
- Ignored real-model GGUF HTTP suites remained ignored by the default workspace
  test command.

## Limits

This slice covers fixture HTTP auth behavior only. It does not rerun ignored
real-model GGUF HTTP suites.
