# Q8_K Comparison Finite Guardrail

Date: 2026-06-28

## Scope

This slice hardens Q8_K activation-matvec comparison instrumentation. The
comparison path is used to measure drift between the default matvec result and
the experimental Q8_K activation-matvec candidate during profiling.

## Red

`ScalarMatVecComparison::new` initially accepted non-finite values and could
therefore produce misleading max-diff evidence. The reference-side test first
failed:

```sh
cargo test -p ferrite-inference matvec_comparison_rejects_non_finite_values -- --nocapture
```

Failure:

```text
Error: InferenceError { message: "non-finite comparison must fail" }
```

The candidate-side test also failed while the candidate guard was absent:

```sh
cargo test -p ferrite-inference matvec_comparison_rejects_non_finite_candidate_values -- --nocapture
```

Failure:

```text
Error: InferenceError { message: "non-finite candidate comparison must fail" }
```

## Green

`ScalarMatVecComparison::new` now rejects non-finite reference and candidate
values before computing absolute or relative drift.

Focused verification:

```sh
cargo test -p ferrite-inference matvec_comparison_rejects_non_finite -- --nocapture
cargo test -p ferrite-inference --test scalar_profile -- --nocapture
cargo test -p ferrite-inference q8_k -- --nocapture
```

Results:

```text
matvec_comparison_rejects_non_finite_values ... ok
matvec_comparison_rejects_non_finite_candidate_values ... ok
token_id_only_profile_records_output_argmax_matrix ... ok
q8_k: 34 passed
```

## Interpretation

This does not change Q8_K dispatch policy. It makes Path B comparison evidence
fail closed if either side of the measured matvec result contains non-finite
values.
