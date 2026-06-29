# Q8_K Comparison Argmax Diagnostic

Date: 2026-06-29

## Scope

Path B Q8_K activation-matvec comparison output now records the argmax index
chosen by the default reference output and by the Q8_K candidate output.

The existing comparison metric showed the maximum absolute and relative drift,
but it did not show whether that drift changed the row or token selected by a
matrix output. This made narrow-margin divergences harder to classify during
Path B probes.

The CLI comparison line now appends:

- reference argmax index;
- candidate argmax index.

This remains diagnostic-only. It does not enable Q8_K activation matvecs by
default and does not promote Path B beyond its experimental parity-scoped
policy.

## Red

Core comparison test:

```sh
cargo test -p ferrite-inference matvec_comparison_records_argmax_indexes -- --nocapture
```

Expected failure before implementation:

```text
no method named `reference_argmax_index` found
no method named `candidate_argmax_index` found
```

CLI shape regression:

```sh
cargo test -p ferrite-cli cli_compares_q8_k_activation_matvec_without_changing_execution_policy -- --nocapture
```

Expected failure before implementation:

```text
assertion failed: q8_k_compare_line_has_argmax_indexes(...)
```

## Green

`ScalarMatVecComparison` now stores `reference_argmax_index` and
`candidate_argmax_index`. The CLI appends both indexes to the existing
`profile_*_q8_k_compare` line after the drift metrics.

Focused checks:

```sh
cargo test -p ferrite-inference matvec_comparison_records_argmax_indexes -- --nocapture
cargo test -p ferrite-cli cli_compares_q8_k_activation_matvec_without_changing_execution_policy -- --nocapture
```

Observed result:

- both focused tests passed.

## Interpretation

This slice improves Path B falsification and debugging. If a Q8_K candidate has
large drift but the same argmax index, it is a magnitude-only warning for that
matrix output. If the candidate argmax differs, the comparison line now marks
the local decision change directly.
