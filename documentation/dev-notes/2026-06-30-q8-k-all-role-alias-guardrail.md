# Q8_K All Role Alias Guardrail

Date: 2026-06-30

## Context

Path B remains an explicit, opt-in Q8_K activation-matvec experiment. Its role
scope parser accepts `all` as a stable CLI alias for every Q8_K activation
role, and accepts comma-separated named roles for narrower diagnostics.

Before this slice, a mixed value such as `all,ffn_up` failed only because
`all` was parsed as an unknown named role. That kept dispatch safe, but the
error did not directly state the role-scope invariant: `all` is a complete
scope alias and must not be combined with named roles.

## Change

- Added a parser guardrail that rejects mixed `all` alias lists explicitly.
- Added a unit test for `Q8KActivationMatvecRole::parse_list("all,ffn_up")`.
- Left Path B dispatch policy unchanged; default execution remains
  `default_only`, and Q8_K activation matvec remains experimental and
  parity-scoped.

## Verification

Red check before implementation:

```text
cargo test -p ferrite-inference q8_k_activation_roles_reject_mixed_all_alias -- --nocapture

test scalar::options::tests::q8_k_activation_roles_reject_mixed_all_alias ... FAILED
left: "unknown Q8_K activation matvec role all"
right: "Q8_K activation matvec role alias all cannot be combined with other roles"
```

Green checks after implementation:

```text
cargo test -p ferrite-inference q8_k_activation_roles_reject_mixed_all_alias -- --nocapture

test scalar::options::tests::q8_k_activation_roles_reject_mixed_all_alias ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 87 filtered out
```

```text
cargo test -p ferrite-inference q8_k_activation_roles -- --nocapture

test scalar::options::tests::q8_k_activation_roles_reject_mixed_all_alias ... ok
test scalar::options::tests::q8_k_activation_roles_reject_duplicate_cli_names ... ok
test scalar::options::tests::q8_k_activation_roles_parse_stable_cli_names ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 85 filtered out
```

```text
cargo test -p ferrite-cli q8_k -- --nocapture

test args::tests::rejects_unknown_q8_k_activation_roles_before_required_inputs ... ok
test cli_compares_q8_k_activation_matvec_for_benchmark_token_profile ... ok
test cli_rejects_q8_k_role_scope_without_comparison_or_experimental_dispatch ... ok
test cli_enables_experimental_q8_k_activation_matvec ... ok
test cli_scopes_experimental_q8_k_activation_matvec_roles ... ok
test cli_scopes_q8_k_comparison_roles_without_changing_execution_policy ... ok
test cli_accepts_all_q8_k_activation_role_scope ... ok
test cli_compares_q8_k_activation_matvec_without_changing_execution_policy ... ok
```

## Interpretation

This closes a small CLI/API clarity hole in the approved Path B control
surface. It does not make Path B default-eligible, does not change kernel
arithmetic, and does not add new model-output or benchmark evidence.
