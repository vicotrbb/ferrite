# 2026-06-27 CLI Benchmark Session Slice

## Scope

This slice changes `--benchmark-runs` to reuse a scalar session so repeated
benchmark calls measure incremental token acceptance with cached K/V state.

## Implementation

- The CLI now starts a `ScalarLlamaSession` for the prompt path.
- The initial next-token output is produced by `accept_prompt`.
- Benchmark iterations call `accept_token` with the previous generated token.
- Added `benchmark_cached_tokens` to make cache growth visible in output.

## Boundaries

This does not add sampling, stop conditions, chat templates, or production
generation loops. It only makes the existing benchmark mode align with the
incremental scalar session boundary.

## Evidence

- Red: `cargo test -p ferrite-cli --test next_token_cli
  cli_benchmarks_repeated_next_token_runs_after_loading_once` failed because
  output did not include `benchmark_cached_tokens=4`.
- Green: the same targeted test passed after benchmark mode reused a session
  and reported cached token count.
