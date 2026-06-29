# OpenAI CORS Module Split

Date: 2026-06-29

## Scope

This slice moved OpenAI CORS and preflight helpers from
`crates/ferrite-server/src/openai/routes.rs` into the focused private
`crates/ferrite-server/src/openai/cors.rs` module.

`routes.rs` still owns router wiring and endpoint handler orchestration. The
new CORS module owns:

- OpenAI endpoint OPTIONS preflight responses;
- `access-control-allow-origin`;
- `access-control-allow-methods`;
- `access-control-allow-headers`;
- attaching CORS headers to `/v1/*` responses.

This is a production-code organization slice only. It does not change route
paths, preflight status codes, CORS header values, authentication policy,
handler behavior, or inference execution.

## Verification

Before the move:

```sh
cargo test -p ferrite-server --lib openai::auth_tests::openai_cors_preflight_does_not_require_bearer_token -- --nocapture
cargo test -p ferrite-server --lib openai::auth_tests::protected_openai_routes_include_cors_response_header -- --nocapture
cargo test -p ferrite-server --lib openai::auth_tests:: -- --nocapture
```

Observed results:

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 254 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 254 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 247 filtered out; finished in 0.02s
```

After the move:

```sh
cargo test -p ferrite-server --lib openai::auth_tests::openai_cors_preflight_does_not_require_bearer_token -- --nocapture
cargo test -p ferrite-server --lib openai::auth_tests::protected_openai_routes_include_cors_response_header -- --nocapture
cargo test -p ferrite-server --lib openai::auth_tests:: -- --nocapture
```

Observed results:

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 254 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 254 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 247 filtered out; finished in 0.00s
```

`routes.rs` now contains 269 lines, while `cors.rs` contains 36 lines.
