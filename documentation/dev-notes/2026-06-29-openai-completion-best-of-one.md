# OpenAI Completion Best Of One

Date: 2026-06-29

## Scope

Accept legacy completion requests with `best_of: 1` as a local no-op while
continuing to reject `best_of` values that would require generating multiple
server-side candidate completions.

This is an OpenAI-compatible HTTP request-shape slice only. It does not
implement multi-candidate generation, log-probability ranking, or `best_of`
with streaming.

Source reference:

- https://developers.openai.com/api/reference/resources/completions/methods/create/

## Rationale

The OpenAI Completions reference describes `best_of` as generating multiple
candidate completions server-side and returning the best candidate. Ferrite's
local legacy completion endpoint currently generates one deterministic local
completion per prompt. `best_of: 1` therefore matches the existing local
execution shape, while `best_of: 2` or higher would require work Ferrite does
not implement.

Accepting the no-op value improves compatibility with clients that send the
default explicitly without pretending Ferrite supports server-side candidate
ranking.

## Change

- Added fixture coverage that accepts `best_of: 1` on `/v1/completions`.
- Added explicit rejection coverage for `best_of: 2`.
- Changed completion request unsupported-field validation so `best_of` uses
  the existing neutral-number helper with expected value `1`.

## Red Tests

```sh
cargo test -p ferrite-server --lib openai::completion_option_tests -- --nocapture
```

Observed result before implementation:

- `completions_endpoint_accepts_single_best_of_candidate` failed with HTTP
  `400` and `error.param = "best_of"`.
- The rest of the suite passed.

## Validation

Post-implementation validation:

```sh
cargo test -p ferrite-server --lib openai::completion_option_tests -- --nocapture
cargo test -p ferrite-server --lib openai::completion_unsupported_tests -- --nocapture
cargo fmt --all -- --check
git diff --check
CARGO_BUILD_JOBS=2 cargo clippy -p ferrite-server --all-targets -- -D warnings
```

Observed result:

- `openai::completion_option_tests`: 8 passed.
- `openai::completion_unsupported_tests`: 9 passed.
- Formatting check passed.
- Whitespace check passed.
- `ferrite-server` clippy passed with warnings denied.

## Limits

This slice does not add multi-candidate completion ranking and does not rerun
ignored real-model GGUF HTTP suites.
