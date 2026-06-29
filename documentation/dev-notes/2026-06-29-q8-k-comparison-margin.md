# Q8_K Comparison Margin Diagnostic

Date: 2026-06-29

## Scope

Path B Q8_K activation-matvec comparison output now reports argmax margins
alongside argmax indexes.

The previous diagnostic line identified whether the reference output and Q8_K
candidate output chose different argmax indexes. It did not quantify how close
each decision was. The margin fields make narrow decision flips explicit.

The CLI comparison line now ends with:

- reference argmax index;
- candidate argmax index;
- reference argmax margin;
- candidate argmax margin.

The margin is the top output value minus the runner-up output value for the
same vector. Single-value outputs use a finite `0.0` margin because there is no
runner-up row.

## Red

Core comparison test:

```sh
cargo test -p ferrite-inference matvec_comparison_records_argmax_indexes_and_margins -- --nocapture
```

Expected failure before implementation:

```text
no method named `reference_argmax_margin` found
no method named `candidate_argmax_margin` found
```

CLI shape regression:

```sh
cargo test -p ferrite-cli cli_compares_q8_k_activation_matvec_without_changing_execution_policy -- --nocapture
```

Expected failure before implementation:

```text
assertion failed: q8_k_compare_line_has_argmax_indexes_and_margins(...)
```

## Green

`ScalarMatVecComparison` now stores reference and candidate argmax margins, and
the CLI appends both margin fields after the argmax indexes.

Focused checks:

```sh
cargo test -p ferrite-inference matvec_comparison_records_argmax_indexes_and_margins -- --nocapture
cargo test -p ferrite-cli cli_compares_q8_k_activation_matvec_without_changing_execution_policy -- --nocapture
```

Observed result:

- both focused tests passed.

## Real Qwen Probe

Command:

```sh
cargo run --release -p ferrite-cli -- --model target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf --prompt-token-ids 21605,6832,4119,646,387 --profile-next-token --experimental-q8-k-activation-matvec --compare-q8-k-activation-matvec --top-logits 5
```

Selected output:

```text
experimental_q8_k_activation_matvec=true
compare_q8_k_activation_matvec=true
q8_k_activation_matvec_policy=experimental_parity_scoped
q8_k_activation_matvec_roles=all
next_token_id=16176
profile_next_token_q8_k_compare=output:Q6_K:151936:1536:191439360:0.795129:9687.982422:1483:16176:0.297201:0.063408
top_logits=16176:20.969004,1483:20.905596,21091:19.688211,17779:19.472042,26075:19.429556
```

Interpretation:

- The default reference output's top-vs-runner-up margin on the experimental
  hidden state was `0.297201`.
- The Q8_K candidate output's top-vs-runner-up margin was only `0.063408`.
- This confirms the final decision flip is narrow on the Q8_K candidate side.

## Boundary

This slice improves diagnostics only. It does not change execution policy,
matrix arithmetic, or dispatch eligibility. Path B remains experimental until
role-scoped or arithmetic changes have model-output parity evidence.
