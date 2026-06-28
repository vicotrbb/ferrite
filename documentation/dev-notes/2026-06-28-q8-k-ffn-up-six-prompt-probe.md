# Q8_K FFN-Up Six-Prompt Probe

Date: 2026-06-28

## Scope

This note follows `documentation/dev-notes/2026-06-28-q8-k-single-role-probes.md`.
That slice left `ffn_up` as the only single-role Q8_K activation candidate that
matched the first two fixed SmolLM2-1.7B-Instruct Q4_K_M prompts. This slice
tests `ffn_up` against the existing six fixed SmolLM2 prompt profiles.

## Result Matrix

| Prompt | Expected IDs | `ffn_up` IDs | Result |
| --- | --- | --- | --- |
| `hello world` | `18,198,3725,198,198,788` | `18,198,3725,198,198,788` | Matched |
| `The capital of France is` | `7042,30,2` | `7042,30,2` | Matched |
| `Once upon a time` | `28,281,253,1165,6560,32047` | `28,281,253,1165,6560,32047` | Matched |
| `Rust is a systems programming language` | `338,2433,253,1837,3500,1743` | `338,2433,253,1837,3500,1743` | Matched |
| `Machine learning models can` | `597,325,804,288,6524,260` | `325,804,288,6524,260,940` | Diverged |
| `The recipe calls for` | `216,34,12382,282,7367,30` | `216,34,12382,282,7367,28` | Diverged |

The `ffn_up` policy was:

```text
experimental_q8_k_activation_matvec=true
q8_k_activation_matvec_policy=experimental_parity_scoped
q8_k_activation_matvec_roles=ffn_up
```

## Baseline Recheck

The two failed prompts were rerun with the default policy and explicit expected
IDs. Both default-policy runs still matched:

```text
prompt=Machine learning models can
q8_k_activation_matvec_policy=default_only
generated_token_ids=597,325,804,288,6524,260
generated_match=true
```

```text
prompt=The recipe calls for
q8_k_activation_matvec_policy=default_only
generated_token_ids=216,34,12382,282,7367,30
generated_match=true
```

## Conclusion

The broader six-prompt check rejects `ffn_up` as a default-dispatch candidate
for SmolLM2-1.7B. It matched four fixed prompts and diverged on two, including a
first-token divergence for `Machine learning models can`.

Path B remains useful as an opt-in kernel-contract and profiling path, but the
role-scope policy evidence no longer leaves a tested default-safe role subset
for this model. The next correctness direction should be a tighter activation
quantization strategy or a model/role policy that is explicitly opt-in and
documented as non-default.
