# OpenAI Auth Module Split

Date: 2026-06-29

## Scope

This slice moved OpenAI bearer-token authorization helpers from
`crates/ferrite-server/src/openai/routes.rs` into the focused private
`crates/ferrite-server/src/openai/auth.rs` module.

`routes.rs` still owns endpoint routing and handler orchestration. The new auth
module owns:

- optional API-key enforcement;
- Authorization header extraction;
- case-insensitive Bearer scheme matching;
- repeated whitespace handling in Bearer tokens.

This is a production-code organization slice only. It does not change
authentication policy, protected route coverage, CORS behavior, method errors,
unknown route errors, routing, or inference execution.

## Verification

Before the move:

```sh
cargo test -p ferrite-server --lib openai::auth_tests:: -- --nocapture
cargo test -p ferrite-server --lib openai::request_error_tests:: -- --nocapture
```

Observed results:

```text
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 247 filtered out; finished in 0.00s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 251 filtered out; finished in 0.01s
```

After the move:

```sh
cargo test -p ferrite-server --lib openai::auth_tests:: -- --nocapture
cargo test -p ferrite-server --lib openai::request_error_tests:: -- --nocapture
```

Observed results:

```text
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 247 filtered out; finished in 0.01s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 251 filtered out; finished in 0.01s
```

`routes.rs` now contains 326 lines, while `auth.rs` contains 40 lines.
