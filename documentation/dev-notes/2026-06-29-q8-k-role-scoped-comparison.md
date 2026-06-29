# Q8_K Role-Scoped Comparison Guardrail

Date: 2026-06-29

## Scope

This slice closes a Path B diagnostic hole in the CLI.

`--experimental-q8-k-activation-roles` now scopes Q8_K activation-matvec roles
without changing the main execution policy by itself. Operators must pass
`--experimental-q8-k-activation-matvec` explicitly when they want role-scoped
experimental dispatch. Comparison-only profiling can now combine
`--compare-q8-k-activation-matvec` with a role scope and still observe the
default execution path.

## Why

The non-invasive comparison guardrail says Q8_K comparison is diagnostic
instrumentation: it should measure a Q8_K candidate separately without changing
execution policy unless the operator explicitly enables experimental dispatch.
Before this slice, the role-scope flag still implied experimental execution,
which made scoped comparison-only probes invasive.

## Red

The new CLI regression first asserted that role-scoped comparison keeps default
execution:

```sh
cargo test -p ferrite-cli cli_scopes_q8_k_comparison_roles_without_changing_execution_policy -- --nocapture
```

Expected failure before implementation:

```text
assertion failed: stdout.contains("experimental_q8_k_activation_matvec=false")
```

## Green

- Removed the implicit `experimental_q8_k_activation_matvec = true` assignment
  from `--experimental-q8-k-activation-roles`.
- Updated the explicit experimental role-scoped CLI test to pass
  `--experimental-q8-k-activation-matvec`.
- Added comparison-only role-scope coverage that verifies:
  - `experimental_q8_k_activation_matvec=false`
  - `q8_k_activation_matvec_policy=default_only`
  - `q8_k_activation_matvec_roles=ffn_down`
  - only `ffn_down` comparison rows are emitted

Focused checks:

```sh
cargo test -p ferrite-cli cli_scopes_q8_k_comparison_roles_without_changing_execution_policy -- --nocapture
cargo test -p ferrite-cli q8_k_activation_matvec -- --nocapture
```

## Interpretation

Path B remains opt-in and parity-scoped. This slice makes role scoping safe for
diagnostics: a role filter no longer changes execution policy unless the
operator also passes the explicit experimental execution flag.
