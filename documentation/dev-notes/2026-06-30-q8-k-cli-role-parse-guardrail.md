# 2026-06-30 Q8_K CLI Role Parse Guardrail

## Summary

Ferrite now validates `--experimental-q8-k-activation-roles` during CLI
argument parsing and carries the result as typed `Q8KActivationMatvecRole`
values.

Previously, the CLI stored the role list as a raw string and parsed it in the
run layer after model loading. That still rejected invalid roles eventually,
but the argument boundary could advance to unrelated required-input errors
first. Path B is still experimental, so opt-in controls should fail early and
unambiguously.

## Changes

- Parse Q8_K activation role scopes in `args.rs`.
- Store typed role values in `CliArgs`.
- Apply typed roles in `run.rs` without reparsing strings.
- Added a parser unit test proving an unknown Q8_K role is rejected before
  missing model or prompt arguments.

## Verification

Red test before implementation:

```text
cargo test -p ferrite-cli args::tests::rejects_unknown_q8_k_activation_roles_before_required_inputs -- --nocapture
unexpected error: missing --model argument
test args::tests::rejects_unknown_q8_k_activation_roles_before_required_inputs ... FAILED
```

Focused green test after implementation:

```text
cargo test -p ferrite-cli args::tests::rejects_unknown_q8_k_activation_roles_before_required_inputs -- --nocapture
test args::tests::rejects_unknown_q8_k_activation_roles_before_required_inputs ... ok
```
