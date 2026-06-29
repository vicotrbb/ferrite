# Q8_K All Role Scope Alias

Date: 2026-06-29

## Slice

Ferrite's Q8_K activation-matvec CLI now accepts `all` as an explicit
`--experimental-q8-k-activation-roles` value.

The CLI already printed the full default role mask as
`q8_k_activation_matvec_roles=all`, but the parser did not accept that value
back from scripts. This made diagnostic commands less round-trip friendly while
Path B remains experimental.

## Red

The focused CLI integration test first tried:

```sh
cargo test -p ferrite-cli cli_accepts_all_q8_k_activation_role_scope -- --nocapture
```

It failed before implementation with:

```text
unknown Q8_K activation matvec role all
```

## Green

Changes:

- `Q8KActivationMatvecRole::parse_list("all")` now expands to the stable
  canonical role list.
- Existing comma-separated role parsing remains unchanged.
- Default execution remains `default_only`; this alias only affects explicit
  role-scope parsing when comparison or experimental execution is already
  requested.

Verification:

```sh
cargo test -p ferrite-cli cli_accepts_all_q8_k_activation_role_scope -- --nocapture
cargo test -p ferrite-inference q8_k_activation_roles_parse_stable_cli_names -- --nocapture
cargo test -p ferrite-inference q8_k_activation_role_scope_defaults_to_all_roles -- --nocapture
```

All focused tests passed after implementation.

## Boundary

This slice does not add new Q8_K parity evidence, change the default dispatch,
or make Path B non-experimental. It only makes the existing `all` diagnostic
label parseable as an explicit role scope.
