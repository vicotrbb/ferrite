# Q8_K Duplicate Role Guardrail

## Scope

This slice hardens Path B role-scoped diagnostics by rejecting duplicate
`--experimental-q8-k-activation-roles` entries during role-list parsing.

The previous parser accepted repeated roles such as `ffn_up,ffn_up`; the role
mask later collapsed them to one role. That was harmless for execution, but it
made experiment setup less explicit and could hide accidental duplicated CLI
input.

## Red

The new regression first proved that duplicates were accepted:

```sh
cargo test -p ferrite-inference q8_k_activation_roles_reject_duplicate_cli_names -- --nocapture
```

Expected failure before implementation:

```text
called `Result::unwrap_err()` on an `Ok` value: [FfnUp, FfnUp]
```

## Green

`Q8KActivationMatvecRole::parse_list` now tracks parsed roles and returns:

```text
duplicate Q8_K activation matvec role ffn_up
```

Focused checks:

```sh
cargo test -p ferrite-inference q8_k_activation_roles_reject_duplicate_cli_names -- --nocapture
cargo test -p ferrite-inference q8_k_activation_roles -- --nocapture
```

## Interpretation

Path B remains opt-in and parity-scoped. This guardrail only makes role-scoped
Q8_K experiments stricter so repeated role names do not silently collapse into
a different-looking effective scope.
