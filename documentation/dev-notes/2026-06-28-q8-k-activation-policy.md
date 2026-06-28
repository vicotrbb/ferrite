# Q8_K Activation Policy

Date: 2026-06-28

## Scope

This slice makes the Q4_K/Q6_K x Q8_K activation matvec approval boundary
explicit in code.

Path B remains approved only as an opt-in, parity-scoped kernel-contract path.
Default execution still uses the existing Q4_K/Q6_K routes. This slice does not
promote Q8_K activation matvecs to default dispatch and does not claim new
throughput.

## Red-Green Evidence

The policy tests were written first and failed because the explicit policy type
and methods did not exist:

```text
no `Q8KActivationMatvecPolicy` in `scalar::options`
no method named `q8_k_activation_matvec_policy`
no method named `with_q8_k_activation_matvec_policy`
```

The CLI test then failed because runs did not print the policy name:

```text
assertion failed: stdout.contains("q8_k_activation_matvec_policy=experimental_parity_scoped")
```

## Implementation

`ScalarExecutionOptions` now stores:

```text
Q8KActivationMatvecPolicy::DefaultOnly
Q8KActivationMatvecPolicy::ExperimentalParityScoped
```

The existing `with_q8_k_activation_matvec(bool)` adapter remains for current
callers, but maps to the explicit policy. The CLI builds the explicit policy and
prints one of these stable names:

```text
q8_k_activation_matvec_policy=default_only
q8_k_activation_matvec_policy=experimental_parity_scoped
```

This makes benchmark and model-output logs self-describing without changing the
dispatch gate.

## Verification

Focused checks passed:

```sh
cargo test -p ferrite-inference policy -- --nocapture
cargo test -p ferrite-cli cli_enables_experimental_q8_k_activation_matvec -- --nocapture
cargo test -p ferrite-cli cli_loads_gguf_and_prints_text_prompt_next_token -- --nocapture
```

The focused output included:

```text
default_policy_keeps_q8_k_activation_matvec_disabled ... ok
parity_scoped_policy_enables_q8_k_activation_matvec ... ok
legacy_bool_adapter_maps_to_explicit_q8_k_policy ... ok
q8_k_activation_matvec_policy_has_stable_output_names ... ok
cli_enables_experimental_q8_k_activation_matvec ... ok
cli_loads_gguf_and_prints_text_prompt_next_token ... ok
```

## Conclusion

The Path B approval boundary is now represented in code and CLI evidence. The
sound state is:

- default policy: do not use Q8_K activation matvecs;
- experimental parity-scoped policy: allow the opt-in Q8_K route where the
  caller is deliberately gathering or relying on scoped parity evidence.

SmolLM2-1.7B parity failure remains a blocker for default promotion.
