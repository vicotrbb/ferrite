# Q8_K Role Scope No-Op Guardrail

Date: 2026-06-29

## Scope

This slice prevents `--experimental-q8-k-activation-roles` from being accepted
as a no-op.

Role scoping is meaningful only when it filters experimental Q8_K execution or
Q8_K comparison diagnostics. The CLI now rejects a bare role scope unless it is
paired with either `--experimental-q8-k-activation-matvec` or
`--compare-q8-k-activation-matvec`.

## Red

The new CLI regression first proved that a bare role scope succeeded:

```sh
cargo test -p ferrite-cli cli_rejects_q8_k_role_scope_without_comparison_or_experimental_dispatch -- --nocapture
```

Expected failure before implementation:

```text
assertion failed: !output.status.success()
```

## Green

`validate_modes` now rejects role scoping unless it has a consumer:

- explicit experimental dispatch; or
- comparison diagnostics.

Focused checks:

```sh
cargo test -p ferrite-cli cli_rejects_q8_k_role_scope_without_comparison_or_experimental_dispatch -- --nocapture
cargo test -p ferrite-cli q8_k_activation_matvec -- --nocapture
```

## Interpretation

Path B remains opt-in and diagnostic-friendly. This guardrail keeps CLI output
from implying a role-scoped experiment when neither experimental execution nor
comparison instrumentation is active.
