# OpenAI Authenticated Client Proof

Date: 2026-06-28

## Summary

Ferrite's optional bearer-token policy is now covered by live client tests.

The `async-openai` integration test starts Ferrite with an API key, configures
the OpenAI client with the same key and a local Ferrite base URL, then performs
a typed chat completion request. This proves the standard OpenAI client path
uses a bearer token compatible with Ferrite's `/v1/*` auth guard.

The raw HTTP integration test also starts an auth-enabled Ferrite server and
sends a matching `Authorization: Bearer ...` header.

## Implementation Notes

- Extended `tests/support::LiveServer` with `start_with_api_key`.
- Added `async_openai_client_uses_api_key_as_ferrite_bearer_token`.
- Added `live_http_server_accepts_matching_bearer_token`.
- Kept production auth code unchanged; this slice only strengthens live
  compatibility proof.

## Verification

Red test first:

```sh
cargo test -p ferrite-server --test openai_client async_openai_client_uses_api_key_as_ferrite_bearer_token -- --nocapture
```

Initial result before support implementation:

- compile failed because `LiveServer::start_with_api_key` did not exist.

Final verification:

```sh
cargo fmt --all
cargo test -p ferrite-server -- --nocapture
cargo clippy -p ferrite-server --all-targets -- -D warnings
cargo check --workspace
git diff --check
```

Observed result:

- `cargo test -p ferrite-server -- --nocapture`: 32 unit tests passed,
  2 `openai_client` integration tests passed, and 2 `openai_http` integration
  tests passed.
- `cargo clippy -p ferrite-server --all-targets -- -D warnings`: passed after
  using `start_with_api_key` from both integration-test modules.
- `cargo check --workspace`: passed.
- `git diff --check`: passed.
