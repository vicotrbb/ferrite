# OpenAI Server Backpressure

Date: 2026-06-28

## Summary

Ferrite's OpenAI-compatible server now enforces bounded inference execution.
Only one request may hold the inference permit at a time. If another generation
request arrives while the permit is held, the server returns an OpenAI-shaped
`rate_limit_error` with HTTP `429`.

This makes the Phase 5 API roadmap requirement explicit: request backpressure
is enforced before a request enters the blocking scalar generation path.

## Implementation Notes

- `ServerState` owns a one-permit semaphore shared by cloned router state.
- Non-streaming and streaming routes acquire the permit before starting
  generation.
- Streaming routes move the permit into the blocking generation task so it is
  held until token streaming completes.
- The server returns `rate_limit_error` instead of queueing unbounded blocking
  tasks behind the model mutex.

## Verification

Red tests first:

```sh
cargo test -p ferrite-server -- state::tests::inference_permit_rejects_second_holder_until_released openai::routes_tests::completions_endpoint_returns_429_when_inference_is_busy -- --nocapture
```

Initial result:

- both tests failed because `ServerState::try_acquire_inference_permit` did not
  exist.

Final verification:

```sh
cargo test -p ferrite-server -- --nocapture
cargo clippy -p ferrite-server --all-targets -- -D warnings
cargo check --workspace
git diff --check
```

Observed result:

- `cargo test -p ferrite-server -- --nocapture`: 15 passed.
- `cargo clippy -p ferrite-server --all-targets -- -D warnings`: passed.
- `cargo check --workspace`: passed.
- `git diff --check`: passed.
