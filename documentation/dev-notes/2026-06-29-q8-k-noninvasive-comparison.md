# Q8_K Non-Invasive Comparison Guardrail

Date: 2026-06-29

## Scope

This slice decouples Q8_K activation-matvec comparison from Q8_K execution.

`--compare-q8-k-activation-matvec` is diagnostic instrumentation: it should
measure drift between the default matvec result and an explicit Q8_K candidate
without changing the main execution policy unless the operator also passes
`--experimental-q8-k-activation-matvec`.

## Red

The CLI regression first asserted that comparison-only profiling keeps default
execution while still emitting Q8_K comparison rows:

```sh
cargo test -p ferrite-cli cli_compares_q8_k_activation_matvec_without_changing_execution_policy -- --nocapture
```

Failure:

```text
assertion failed: stdout.contains("experimental_q8_k_activation_matvec=false")
```

The failure proved that the comparison flag still implied experimental Q8_K
execution.

## Green

The CLI parser no longer enables `--experimental-q8-k-activation-matvec` when
`--compare-q8-k-activation-matvec` is present.

The scalar profiling path now computes the Q8_K comparison candidate explicitly
with an internal candidate option. Role scoping still controls which roles emit
comparison records, but comparison no longer requires the main execution policy
to be experimental.

## Verification

```sh
cargo test -p ferrite-cli cli_compares_q8_k_activation_matvec_without_changing_execution_policy -- --nocapture
cargo test -p ferrite-cli q8_k_activation_matvec -- --nocapture
cargo test -p ferrite-inference q8_k_activation -- --nocapture
```

Results:

```text
cli_compares_q8_k_activation_matvec_without_changing_execution_policy ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 21 filtered out

cli_enables_experimental_q8_k_activation_matvec ... ok
cli_scopes_experimental_q8_k_activation_matvec_roles ... ok
cli_compares_q8_k_activation_matvec_without_changing_execution_policy ... ok
cli_compares_q8_k_activation_matvec_for_benchmark_token_profile ... ok
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 18 filtered out

q8_k_activation: 7 passed
```

## Interpretation

This does not promote Q8_K activation matvecs to default dispatch. It makes Path
B drift evidence easier to trust because comparison-only profiling now observes
default execution and measures the Q8_K candidate separately.
