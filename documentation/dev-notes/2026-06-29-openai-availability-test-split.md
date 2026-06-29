# OpenAI Availability Test Split

Date: 2026-06-29

## Scope

This slice moved three generation availability and model-resolution tests from
`crates/ferrite-server/src/openai/routes_tests.rs` into
`crates/ferrite-server/src/openai/availability_tests.rs`:

- `chat_endpoint_returns_openai_error_when_model_is_not_loaded`;
- `completions_endpoint_returns_model_not_found_for_unknown_model`;
- `chat_endpoint_returns_model_not_found_for_unknown_model`.

This is a test-organization slice only. It does not change server routing,
request schemas, response schemas, model loading, inference execution, or error
semantics.

## Motivation

`availability_tests.rs` already owns checks that server-side generation
availability is reported before queue pressure. Moving unloaded-model and
unknown-model generation tests there keeps engine/model availability behavior
in one focused module and continues reducing the broad route test surface.

## Verification

Before the move:

```sh
cargo test -p ferrite-server --lib openai::routes_tests::chat_endpoint_returns_openai_error_when_model_is_not_loaded -- --nocapture
cargo test -p ferrite-server --lib openai::routes_tests::completions_endpoint_returns_model_not_found_for_unknown_model -- --nocapture
cargo test -p ferrite-server --lib openai::routes_tests::chat_endpoint_returns_model_not_found_for_unknown_model -- --nocapture
cargo test -p ferrite-server --lib openai::availability_tests:: -- --nocapture
```

Observed results:

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 254 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 254 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 254 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 252 filtered out; finished in 0.00s
```

After the move:

```sh
cargo test -p ferrite-server --lib openai::availability_tests:: -- --nocapture
cargo test -p ferrite-server --lib openai::routes_tests:: -- --nocapture
```

Observed results:

```text
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 249 filtered out; finished in 0.00s
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 232 filtered out; finished in 0.04s
```

`routes_tests.rs` now contains 576 lines, while `availability_tests.rs`
contains 152 lines.
