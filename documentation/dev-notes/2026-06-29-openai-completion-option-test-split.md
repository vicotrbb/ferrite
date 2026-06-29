# OpenAI Completion Option Test Split

Date: 2026-06-29

## Scope

This slice moved legacy completion option-acceptance tests from
`crates/ferrite-server/src/openai/routes_tests.rs` into the focused
`crates/ferrite-server/src/openai/completion_option_tests.rs` module.

The moved tests cover local no-op or neutral OpenAI-compatible completion
options:

- neutral sampling fields;
- OpenAI's default `temperature: 1`;
- empty `stop` arrays;
- disabled `echo`;
- empty `logit_bias`;
- `user` identifiers;
- integer `seed`.

This is a test-organization slice only. It does not change request schemas,
response schemas, routing, model loading, sampling behavior, or inference
execution.

## Verification

Before the move:

```sh
cargo test -p ferrite-server --lib openai::routes_tests::completions_endpoint_accepts -- --nocapture
```

Observed result:

```text
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 247 filtered out; finished in 0.02s
```

The baseline command also included
`completions_endpoint_accepts_array_of_string_prompts`, which remains in
`routes_tests.rs` because it is prompt-shape coverage rather than an option
no-op.

After the move:

```sh
cargo test -p ferrite-server --lib openai::completion_option_tests:: -- --nocapture
cargo test -p ferrite-server --lib openai::routes_tests:: -- --nocapture
```

Observed results:

```text
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 248 filtered out; finished in 0.01s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 239 filtered out; finished in 0.03s
```

`routes_tests.rs` now contains 419 lines, while
`completion_option_tests.rs` contains 85 lines.
