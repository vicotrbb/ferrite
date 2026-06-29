# OpenAI Bearer Separator Spaces

Date: 2026-06-29

## Scope

Accept one or more spaces between the `Bearer` auth scheme and token on
protected OpenAI-compatible local server routes.

This is an HTTP compatibility slice only. It does not change API-key storage,
token values, model loading, inference execution, or the unauthenticated
`/health` readiness path.

## Rationale

Ferrite already accepts bearer schemes case-insensitively. The parser still
used `split_once(' ')`, so `Authorization: Bearer   local-secret` was rejected
even though HTTP bearer credentials allow one or more spaces between the scheme
and credential. Accepting repeated separator spaces improves local OpenAI client
compatibility while keeping token comparison exact.

## Change

- Added a focused auth regression for `Authorization: Bearer   local-secret`.
- Replaced single-space splitting with whitespace-token parsing.
- Kept the parser strict about exactly two fields:
  - scheme;
  - token.
- Kept the token comparison exact and case-sensitive.

## Verification

RED command:

```sh
cargo test -p ferrite-server openai::auth_tests -- --nocapture
```

Observed result before the route change:

- `protected_openai_routes_accept_repeated_bearer_separator_spaces` failed:
  expected `200`, got `401`.

GREEN command:

```sh
cargo fmt --all
cargo test -p ferrite-server openai::auth_tests -- --nocapture
```

Observed result:

- `openai::auth_tests`: 4 passed, 0 failed.

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
- Workspace tests passed; `ferrite-server` reported 227 library tests passed,
  including the repeated-space bearer regression.
- Ignored real-model GGUF HTTP suites remained ignored by the default workspace
  test command.

## Limits

This slice covers fixture HTTP auth behavior only. It does not rerun ignored
real-model GGUF HTTP suites.
