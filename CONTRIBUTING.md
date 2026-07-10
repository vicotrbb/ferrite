# Contributing to Ferrite

Thank you for improving Ferrite. Correctness, measured performance, clear
documentation, and reviewable safety boundaries are all part of the product.

## Before opening a change

1. Search existing issues and ADRs.
2. Keep the proposal focused on one observable outcome.
3. For architecture or performance work, establish a baseline before editing.
4. Do not add model binaries, private plans, tool state, or unrelated cleanup.

## Local validation

Run the full repository gate:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --locked
python3 scripts/check_docs.py
python3 scripts/eval_test.py
cargo audit --deny warnings
cargo deny --all-features --locked check
cargo machete
cargo tree --duplicates --locked
```

If the additional Cargo tools are unavailable:

```sh
cargo install --locked cargo-audit
cargo install --locked cargo-deny
cargo install --locked cargo-machete
```

## Performance changes

Include comparable before and after artifacts from `scripts/eval.sh`, plus the
model hash, host, commit, build settings, prompt, token counts, worker count,
TTFT, throughput, memory, CPU, and token trace. Report regressions as clearly as
improvements. A single best run is not sufficient.

## Unsafe changes

Keep unsafe code inside architecture-specific modules. Add a reason to the
module allowance and a safety explanation immediately before every unsafe
block. Include reference parity, boundary tests, cross-architecture checks, and
real-model validation for hot kernels.

## Documentation

Do not use em dashes. Update the maintained guide under `docs/` when behavior
changes. Add an ADR for durable decisions and an eval or benchmark artifact for
measured claims. Historical implementation notes are not a substitute for the
user-facing contract.

## Pull requests

Complete the pull request template with exact validation commands and
performance impact. Keep commits reviewable and use clear imperative subjects,
for example `fix(model): reject duplicate tensor names`.

By contributing, you agree that your contribution is licensed under Ferrite's
MIT License.
