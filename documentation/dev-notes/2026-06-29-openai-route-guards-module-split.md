# OpenAI Route Guards Module Split

Date: 2026-06-29

## Scope

Split OpenAI-compatible route validation, model lookup, token-limit
normalization, and inference permit helpers out of `routes.rs` into a focused
private `guards` module.

This is an organization-only slice. It does not change OpenAI-compatible HTTP
paths, request schemas, response shapes, auth behavior, inference execution, or
streaming behavior.

## Rationale

`routes.rs` was accumulating route orchestration and route-local guard logic in
one file. Keeping the route handlers focused on request flow makes later
OpenAI-compatible endpoint work easier to review without letting the route file
turn into a mixed validation and transport module.

## Change

- Added `crates/ferrite-server/src/openai/guards.rs`.
- Moved supported-field checks, model checks, engine lookup, inference permit
  acquisition, and max-token normalization helpers into the new module.
- Registered the private module from `openai/mod.rs`.
- Kept route handlers on the same helper calls through `pub(super)` functions.

## Baseline

Before the refactor, the focused route guard behavior was covered with:

```sh
cargo test -p ferrite-server --lib openai::availability_tests -- --nocapture
cargo test -p ferrite-server --lib openai::token_limit_tests -- --nocapture
cargo test -p ferrite-server --lib openai::completion_concurrency_tests -- --nocapture
cargo test -p ferrite-server --lib openai::unsupported_tests -- --nocapture
cargo test -p ferrite-server --lib openai::completion_unsupported_tests -- --nocapture
```

Observed result:

- `openai::availability_tests`: 6 passed.
- `openai::token_limit_tests`: 6 passed.
- `openai::completion_concurrency_tests`: 2 passed.
- `openai::unsupported_tests`: 9 passed.
- `openai::completion_unsupported_tests`: 8 passed.

## Validation

Post-refactor validation:

```sh
cargo test -p ferrite-server --lib openai::availability_tests -- --nocapture
cargo test -p ferrite-server --lib openai::token_limit_tests -- --nocapture
cargo test -p ferrite-server --lib openai::completion_concurrency_tests -- --nocapture
cargo test -p ferrite-server --lib openai::unsupported_tests -- --nocapture
cargo test -p ferrite-server --lib openai::completion_unsupported_tests -- --nocapture
cargo fmt --all -- --check
git diff --check
CARGO_BUILD_JOBS=2 cargo clippy -p ferrite-server --all-targets -- -D warnings
```

Observed result:

- `openai::availability_tests`: 6 passed.
- `openai::token_limit_tests`: 6 passed.
- `openai::completion_concurrency_tests`: 2 passed.
- `openai::unsupported_tests`: 9 passed.
- `openai::completion_unsupported_tests`: 8 passed.
- Formatting check passed.
- Whitespace check passed.
- `ferrite-server` clippy passed with warnings denied.

## Limits

This slice does not add new OpenAI-compatible endpoint surface area and does
not rerun ignored real-model GGUF HTTP suites.
