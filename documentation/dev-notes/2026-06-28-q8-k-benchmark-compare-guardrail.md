# Q8_K Benchmark Compare Guardrail

Date: 2026-06-28

## Scope

This slice adds CLI coverage for Q8_K activation-matvec comparison output during
benchmark-token profiling.

Next-token profiling already had Q8_K comparison coverage. Benchmark-token
profiling is the hotter optimization surface for the current Tier 1 Path B
work, so it also needs a direct guardrail.

## Red

The new test initially failed because the Q4_K fixture predicted token `1`,
then benchmark replay accepted token `1`, whose fixture embedding row was all
zeros:

```sh
cargo test -p ferrite-cli cli_compares_q8_k_activation_matvec_for_benchmark_token_profile -- --nocapture
```

Failure:

```text
cli failed with stderr: rms_norm scale is zero
```

## Green

The Q4_K/Q6_K fixture token embedding for the predicted `winner` token is now
non-zero, preserving the first-token winner behavior while making repeated-token
benchmark replay valid for the fixture.

Focused verification:

```sh
cargo test -p ferrite-cli cli_compares_q8_k_activation_matvec_for_benchmark_token_profile -- --nocapture
cargo test -p ferrite-cli cli_compares_experimental_q8_k_activation_matvec -- --nocapture
cargo test -p ferrite-cli cli_enables_experimental_q8_k_activation_matvec -- --nocapture
```

Results:

```text
cli_compares_q8_k_activation_matvec_for_benchmark_token_profile ... ok
cli_compares_experimental_q8_k_activation_matvec ... ok
cli_enables_experimental_q8_k_activation_matvec ... ok
```

## Interpretation

This does not change Q8_K dispatch policy. It ensures
`--profile-benchmark-token --compare-q8-k-activation-matvec` emits comparison
evidence on a Q4_K fixture and that the fixture can exercise repeated-token
benchmark replay without entering a zero-norm hidden state.
