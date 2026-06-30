# OpenAI CORS Test Module Split

## Summary

OpenAI-compatible CORS coverage now lives in
`crates/ferrite-server/src/openai/cors_tests.rs` instead of the authentication
test module.

This keeps bearer-token policy tests and browser preflight/header behavior in
separate focused files as the local OpenAI-compatible server surface grows.

## Scope

This is a test organization slice only:

- moved CORS preflight coverage into `cors_tests.rs`;
- kept authentication tests in `auth_tests.rs`;
- registered the new module from `openai/mod.rs`;
- did not change server runtime behavior.

## Validation

```text
cargo test -p ferrite-server --lib openai::cors_tests -- --nocapture
cargo test -p ferrite-server --lib openai::auth_tests -- --nocapture
cargo fmt --all -- --check
git diff --check
```
