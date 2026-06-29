# OpenAI Catalog List Test Split

Date: 2026-06-29

## Scope

This slice moved `models_endpoint_returns_openai_list_shape` from
`crates/ferrite-server/src/openai/routes_tests.rs` into
`crates/ferrite-server/src/openai/catalog_tests.rs`.

This is a test-organization slice only. It does not change server routing,
request schemas, response schemas, model catalog behavior, model loading, or
inference behavior.

## Motivation

`catalog_tests.rs` already owns `/v1/models/:model` retrieval coverage and the
empty loaded-model list case. Keeping the loaded model-list shape test there
keeps catalog behavior in one focused module and continues reducing the broad
route test surface.

## Verification

Before the move:

```sh
cargo test -p ferrite-server --lib openai::routes_tests::models_endpoint_returns_openai_list_shape -- --nocapture
cargo test -p ferrite-server --lib openai::catalog_tests:: -- --nocapture
```

Observed results:

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 254 filtered out; finished in 0.01s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 250 filtered out; finished in 0.00s
```

After the move:

```sh
cargo test -p ferrite-server --lib openai::catalog_tests:: -- --nocapture
cargo test -p ferrite-server --lib openai::routes_tests:: -- --nocapture
```

Observed results:

```text
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 249 filtered out; finished in 0.00s
test result: ok. 26 passed; 0 failed; 0 ignored; 0 measured; 229 filtered out; finished in 0.03s
```

`routes_tests.rs` now contains 643 lines, while `catalog_tests.rs` contains 154
lines.
