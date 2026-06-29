# OpenAI Catalog Route Module Split

Date: 2026-06-29

## Scope

This slice moved the OpenAI health and model catalog handlers from
`crates/ferrite-server/src/openai/routes.rs` into the focused private
`crates/ferrite-server/src/openai/catalog.rs` module.

`routes.rs` still owns the router wiring and generation endpoint orchestration.
The new catalog module owns:

- `GET /health`;
- `GET /v1/models`;
- `GET /v1/models/{model}`;
- catalog-route authorization checks.

This is a production-code organization slice only. It does not change route
paths, response JSON shape, health readiness semantics, model-not-found
behavior, authentication policy, CORS behavior, or inference execution.

## Verification

Before the move:

```sh
cargo test -p ferrite-server --lib openai::health_tests:: -- --nocapture
cargo test -p ferrite-server --lib openai::catalog_tests:: -- --nocapture
cargo test -p ferrite-server --lib openai::auth_tests:: -- --nocapture
```

Observed results:

```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 253 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 249 filtered out; finished in 0.02s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 247 filtered out; finished in 0.00s
```

After the move:

```sh
cargo test -p ferrite-server --lib openai::health_tests:: -- --nocapture
cargo test -p ferrite-server --lib openai::catalog_tests:: -- --nocapture
cargo test -p ferrite-server --lib openai::auth_tests:: -- --nocapture
```

Observed results:

```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 253 filtered out; finished in 0.01s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 249 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 247 filtered out; finished in 0.01s
```

`routes.rs` now contains 299 lines, while `catalog.rs` contains 41 lines.
