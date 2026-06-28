# Q8_K Qwen2.5-1.5B Parity Probe

Date: 2026-06-28

## Scope

This note tests the all-role experimental Q4_K/Q6_K x Q8_K activation policy on
Qwen2.5-1.5B-Instruct Q4_K_M.

The Q8_K policy previously improved local Qwen2.5-1.5B benchmark-token
throughput, but SmolLM2-1.7B role-scope probes rejected default promotion. This
slice checks whether Qwen2.5-1.5B has model-specific six-prompt parity under
the all-role experimental policy.

## Result Matrix

Policy:

```text
experimental_q8_k_activation_matvec=true
q8_k_activation_matvec_policy=experimental_parity_scoped
q8_k_activation_matvec_roles=all
```

| Prompt | Expected IDs | Experimental Q8_K IDs | Result |
| --- | --- | --- | --- |
| `hello world` | `198,9707,11` | `198,9707,11` | Matched |
| `The capital of France is` | `12095,13,576` | `12095,13,576` | Matched |
| `Once upon a time` | `11,1052,572` | `11,1052,572` | Matched |
| `Rust is a systems programming language` | `429,374,6188` | `429,374,6188` | Matched |
| `Machine learning models can` | `387,1483,311,7023,279,28636` | `387,16176,389,264,8045,315` | Diverged |
| `The recipe calls for` | `220,17,25374,315,19828,323` | `220,17,25374,315,19828,323` | Matched |

## Baseline Recheck

The failed prompt was rerun with the default policy and the same binary. It
still matched:

```text
prompt=Machine learning models can
q8_k_activation_matvec_policy=default_only
generated_token_ids=387,1483,311,7023,279,28636
generated_match=true
```

## Conclusion

All-role experimental Q8_K is not a six-prompt parity pass for
Qwen2.5-1.5B-Instruct Q4_K_M. It matched five prompts and diverged on
`Machine learning models can`.

The existing Qwen2.5-1.5B benchmark improvement remains useful opt-in research
evidence, but it is not enough to justify a model-specific default or automatic
policy. Any future Qwen-specific policy still needs either a tighter activation
quantization strategy or broader parity evidence under a narrower policy.
