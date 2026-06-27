# 2026-06-27 CLI Benchmark Runs Slice

## Scope

This slice adds a small CLI benchmark mode for repeated in-process
`next_token_for_prompt` calls after a model has already been loaded.

## Implementation

- Added `--benchmark-runs <count>` to the CLI.
- Validated the run count is greater than zero.
- Kept the existing single next-token output.
- Added benchmark summary keys:
  - `benchmark_runs`
  - `benchmark_total_ns`
  - `benchmark_avg_ns`

## Boundaries

This is a minimal steady-state measurement surface. It does not add sampling,
multi-token generation, warmup controls, memory instrumentation, or a dedicated
benchmark crate.

## Evidence

- Red: `cargo test -p ferrite-cli --test next_token_cli
  cli_benchmarks_repeated_next_token_runs_after_loading_once` failed because
  `--benchmark-runs` was an unknown argument.
- Green: the same targeted test passed after adding the parser field and timed
  repeated run loop.
