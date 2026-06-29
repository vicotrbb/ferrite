# Q8_K Comparison Role Summary

## Slice

This slice adds role-level summaries for Q8_K activation-matvec comparison
diagnostics. The existing per-operation lines remain unchanged. The new summary
line groups comparisons by role, storage kind, shape, and storage bytes:

```text
profile_next_token_q8_k_compare_role=<role>:<storage_kind>:<rows>:<cols>:<storage_bytes>:<comparisons>:<argmax_mismatches>:<max_abs_diff>:<max_relative_diff>:<min_reference_argmax_margin>:<min_candidate_argmax_margin>
```

The goal is diagnostic only. It does not promote Q8_K activation matvecs to
default dispatch, and it does not change inference arithmetic.

## Red

The CLI tests first required summary lines for both next-token and
benchmark-token Q8_K comparison profiles:

```sh
cargo test -p ferrite-cli q8_k_activation_matvec -- --nocapture
```

The expected failure occurred because the CLI emitted only per-operation
comparison rows:

```text
assertion failed: q8_k_compare_role_summary_has_drift_fields(&stdout,
    "profile_next_token_q8_k_compare_role=q_proj:")
assertion failed: q8_k_compare_role_summary_has_drift_fields(&stdout,
    "profile_benchmark_token_q8_k_compare_role=q_proj:")
```

## Green

`crates/ferrite-cli/src/profile.rs` now aggregates existing
`ScalarMatVecComparison` events into role summaries with:

- comparison count;
- argmax mismatch count;
- maximum absolute drift;
- maximum relative drift;
- minimum reference argmax margin; and
- minimum Q8_K candidate argmax margin.

The focused test passed after implementation:

```sh
cargo test -p ferrite-cli q8_k_activation_matvec -- --nocapture
```

Result:

```text
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 20 filtered out
```

## Real Qwen Probe

Model:

```text
target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf
```

Prompt token IDs:

```text
21605,6832,4119,646,387
```

Comparison-only default execution:

```sh
cargo run --release -p ferrite-cli -- \
  --model target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf \
  --prompt-token-ids 21605,6832,4119,646,387 \
  --profile-next-token \
  --compare-q8-k-activation-matvec \
  --top-logits 5
```

Relevant output:

```text
experimental_q8_k_activation_matvec=false
compare_q8_k_activation_matvec=true
q8_k_activation_matvec_policy=default_only
next_token_id=1483
profile_next_token_q8_k_compare=output:Q6_K:151936:1536:191439360:0.795508:162048.156250:1483:1483:0.344479:0.032722
profile_next_token_q8_k_compare_role=q_proj:Q4_K:1536:1536:1327104:28:1:0.071086:193.941010:0.018256:0.003106
profile_next_token_q8_k_compare_role=output:Q6_K:151936:1536:191439360:1:0:0.795508:162048.156250:0.344479:0.032722
top_logits=1483:21.185814,16176:20.841335,21091:19.790606,17779:19.504631,26075:19.173910
```

Experimental all-role Q8_K execution plus comparison:

```sh
cargo run --release -p ferrite-cli -- \
  --model target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf \
  --prompt-token-ids 21605,6832,4119,646,387 \
  --profile-next-token \
  --experimental-q8-k-activation-matvec \
  --compare-q8-k-activation-matvec \
  --top-logits 5
```

Relevant output:

```text
experimental_q8_k_activation_matvec=true
compare_q8_k_activation_matvec=true
q8_k_activation_matvec_policy=experimental_parity_scoped
next_token_id=16176
profile_next_token_q8_k_compare=output:Q6_K:151936:1536:191439360:0.795129:9687.982422:1483:16176:0.297201:0.063408
profile_next_token_q8_k_compare_role=output:Q6_K:151936:1536:191439360:1:1:0.795129:9687.982422:0.297201:0.063408
top_logits=16176:20.969004,1483:20.905596,21091:19.688211,17779:19.472042,26075:19.429556
```

The default comparison-only profile shows that a single operation can have a
local argmax mismatch without changing default execution. The experimental
all-role profile shows the chained Q8_K path still flips the final output
decision for this prompt. This supports keeping Q8_K as opt-in diagnostic
infrastructure while improving the evidence loop around Path B.
