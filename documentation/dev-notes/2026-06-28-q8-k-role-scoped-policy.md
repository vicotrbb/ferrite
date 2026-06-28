# Q8_K Role-Scoped Activation Policy

Date: 2026-06-28

## Scope

This slice adds a role scope to the experimental Q4_K/Q6_K x Q8_K activation
matvec policy.

The prior boundary probes found no localized Path B formula hole, but they also
showed that the all-eligible-matrices policy is too broad for default promotion:
SmolLM2-1.7B diverges from accumulated activation-quantization drift. A
role-scoped policy lets Ferrite run deliberate parity probes such as
`ffn_down`-only or `output`-only without temporary source edits.

## Implementation

- Added `Q8KActivationMatvecRole` in
  `crates/ferrite-inference/src/scalar/options.rs`.
- Added a compact role mask to `ScalarExecutionOptions`.
- Scoped session dispatch for `q_proj`, `k_proj`, `v_proj`, `o_proj`,
  `ffn_gate`, `ffn_up`, `ffn_down`, and `output`.
- Added CLI support for:

```text
--experimental-q8-k-activation-roles <role[,role...]>
```

The existing `--experimental-q8-k-activation-matvec` behavior remains
all-roles by default.

## TDD Evidence

Red checks failed before implementation:

```sh
cargo test -p ferrite-inference q8_k_activation_role -- --nocapture
cargo test -p ferrite-cli cli_scopes_experimental_q8_k_activation_matvec_roles -- --nocapture
```

Expected failures included the missing `Q8KActivationMatvecRole`, missing role
methods on `ScalarExecutionOptions`, and an unknown CLI argument:

```text
no `Q8KActivationMatvecRole` in `scalar::options`
no method named `q8_k_activation_matvec_roles_label`
unknown argument --experimental-q8-k-activation-roles
```

Green checks after implementation:

```sh
cargo test -p ferrite-inference q8_k_activation_role -- --nocapture
cargo test -p ferrite-cli cli_scopes_experimental_q8_k_activation_matvec_roles -- --nocapture
```

The CLI regression proves scoped comparison output includes the selected role
and excludes unselected roles:

```text
q8_k_activation_matvec_roles=ffn_down
profile_next_token_q8_k_compare=layer.0.ffn_down:
```

## Conclusion

Path B remains an opt-in, parity-scoped research path. This slice tightens the
policy design by making the experimental dispatch role-selectable, so future
parity probes can isolate drift by projection role without changing source
code.
