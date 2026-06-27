# 2026-06-27 CLI Benchmark Token Output Slice

## Scope

This slice adds generated-token ID output to CLI benchmark mode so repeated
session runs can be compared against reference runtimes.

## Implementation

- Captured each generated token ID produced inside the `--benchmark-runs`
  session loop.
- Printed `benchmark_token_ids=<id[,id...]>` alongside timing and cache-count
  output.
- Extended the CLI integration test to require one generated token ID per
  benchmark run.

## Boundaries

This only reports token IDs. It does not decode or print generated text,
implement stop conditions, sampling, or chat-template rendering.

## Evidence

- Red: `cargo test -p ferrite-cli --test next_token_cli
  cli_benchmarks_repeated_next_token_runs_after_loading_once` failed with
  `missing benchmark_token_ids`.
- Green: the same targeted test passed after capturing generated token IDs in
  benchmark mode.
