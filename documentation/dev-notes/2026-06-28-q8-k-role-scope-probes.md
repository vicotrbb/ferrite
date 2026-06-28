# Q8_K Role-Scope Probes

Date: 2026-06-28

## Scope

This note records first real-model probes using the role-scoped Path B policy
from `documentation/dev-notes/2026-06-28-q8-k-role-scoped-policy.md`.

The goal was to verify that role scoping is usable for parity investigation and
to test two broad role subsets on SmolLM2-1.7B-Instruct Q4_K_M.

## Baseline

After rebuilding `target/release/ferrite`, the default policy still matched the
two fixed prompts:

```text
prompt=hello world
q8_k_activation_matvec_policy=default_only
q8_k_activation_matvec_roles=all
generated_token_ids=18,198,3725,198,198,788
generated_match=true
```

```text
prompt=The capital of France is
q8_k_activation_matvec_policy=default_only
q8_k_activation_matvec_roles=all
generated_token_ids=7042,30,2
generated_match=true
```

## Role Probes

Q6-like roles:

```sh
target/release/ferrite \
  --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf \
  --prompt 'hello world' \
  --generate-tokens 6 \
  --experimental-q8-k-activation-roles ffn_down,output
```

Result:

```text
q8_k_activation_matvec_roles=ffn_down,output
generated_token_ids=18,198,198,3272,24,2334
generated_stopped_on_eos=false
```

The same role scope matched the EOS-sensitive prompt:

```text
prompt=The capital of France is
q8_k_activation_matvec_roles=ffn_down,output
generated_token_ids=7042,30,2
generated_stopped_on_eos=true
```

Q4-like FFN roles:

```sh
target/release/ferrite \
  --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf \
  --prompt 'hello world' \
  --generate-tokens 6 \
  --experimental-q8-k-activation-roles ffn_gate,ffn_up
```

Result:

```text
q8_k_activation_matvec_roles=ffn_gate,ffn_up
generated_token_ids=18,198,198,19,21367,42
generated_stopped_on_eos=false
```

The same role scope also diverged on the EOS-sensitive prompt:

```text
prompt=The capital of France is
q8_k_activation_matvec_roles=ffn_gate,ffn_up
generated_token_ids=7042,30,378,3575,282,4649
generated_stopped_on_eos=false
```

## Conclusion

The role-scoped policy works as an investigation tool, but these broad role
subsets are not sufficient for default promotion.

- `ffn_down,output` is prompt-sensitive: it preserved the EOS prompt but still
  diverged on `hello world`.
- `ffn_gate,ffn_up` diverged on both fixed prompts.

Path B remains sound as an opt-in kernel-contract path. It is still not
parity-safe as a default dispatch policy for SmolLM2-1.7B. The next probe should
test narrower single-role scopes and compare per-role Q8_K drift metrics before
any broader promotion discussion.
