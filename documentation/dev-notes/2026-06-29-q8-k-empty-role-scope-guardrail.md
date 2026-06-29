# Q8_K Empty Role Scope Guardrail

## Slice

This slice hardens the approved Path B Q8_K activation-dot contract by making
the low-level role-scope builder reject an empty role set.

The CLI parser already rejects an empty
`--experimental-q8-k-activation-roles` value. The remaining hole was direct
programmatic construction through `ScalarExecutionOptions`: an enabled
experimental policy could carry an empty role mask and print a blank diagnostic
label while dispatching no experimental roles.

## Test First

The new test first failed because an empty role set was accepted:

```sh
cargo test -p ferrite-inference q8_k_activation_role_scope_rejects_empty_role_set -- --nocapture
```

Result:

```text
test scalar::options::tests::q8_k_activation_role_scope_rejects_empty_role_set - should panic ... FAILED
note: test did not panic as expected
```

## Change

`Q8KActivationMatvecRoleMask::from_roles` now enforces the invariant:

```text
Q8_K activation matvec role scope must not be empty
```

This keeps Path B role-scoped diagnostics and dispatch in a valid state: either
all roles are selected by default, or a non-empty explicit subset is selected.

## Validation

Focused guardrail run:

```sh
cargo test -p ferrite-inference q8_k_activation_role_scope_rejects_empty_role_set -- --nocapture
```

Result:

```text
test scalar::options::tests::q8_k_activation_role_scope_rejects_empty_role_set - should panic ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 69 filtered out
```

Adjacent Path B option tests:

```sh
cargo test -p ferrite-inference q8_k_activation -- --nocapture
```

Result:

```text
test result: ok. 7 passed; 0 failed; 0 ignored; 63 filtered out
```

## Boundary

This is a defensive API invariant. It does not change Q8_K arithmetic, promote
Q8_K activation matvecs to default dispatch, or make a new throughput claim.
Path B remains an explicit opt-in, parity-scoped research path.
