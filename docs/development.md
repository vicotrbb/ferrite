# Development guide

Ferrite uses Rust 1.96.1 and the 2024 edition. The explicit toolchain and MSRV
keep local development, CI, and package metadata aligned.

## Set up

```sh
git clone https://github.com/vicotrbb/ferrite.git
cd ferrite
rustup show active-toolchain
cargo test --workspace --all-targets --all-features --locked
```

No sibling checkout is required. The optional Locus backend uses the published
`locus-alloc` crate.

## Engineering rules

- Keep safe APIs around unsafe kernels and validate every shape, length, and CPU
  feature before the unsafe boundary.
- Do not add `unwrap`, `expect`, panic paths, TODO macros, debug macros, or
  undocumented unsafe blocks to production code. Workspace lints enforce this.
- Add a reason to every lint allowance. Prefer fixing or narrowing the code.
- Use checked arithmetic for untrusted sizes and allocations.
- Keep error messages specific enough to identify the invalid field or tensor.
- Keep blocking inference work away from async reactor threads.
- Avoid allocations in decode loops unless measurement proves the tradeoff.
- Do not change numerical accumulation order casually.
- Use release binaries and comparable repeated measurements for performance.

## Documentation rules

- Do not use em dashes. Use commas, colons, parentheses, or separate sentences.
- Update maintained guides when behavior changes.
- Add rustdoc for public library APIs, including errors and safety contracts.
- Use ADRs for durable architecture decisions.
- Use benchmark or eval artifacts for measured claims.
- Do not commit private plans, tool state, scratch reports, or model binaries.
- Keep ADRs, curated benchmark conclusions, and current engineering research
  under `docs/`. Use Git history for experiment archaeology.

## Repository changes

Keep each change focused. Run formatting and strict Clippy early, then tests,
docs, and dependency policy. Performance changes also need the eval workflow in
[evaluation and regression gates](evaluation.md).

## Test organization

Cargo test targets are deliberate. Small fixture-backed suites remain separate
when they exercise distinct binaries or operational surfaces. The server's
artifact-gated model cases share the `real_models` harness, which compiles the
server and test support once while retaining an isolated module for each model
and behavior.

List the artifact-gated cases without running them:

```sh
cargo test -p ferrite-server --test real_models -- --list
```

Run one model module by its harness path, for example:

```sh
FERRITE_PHI3_MODEL=/absolute/path/to/Phi-3-mini-4k-instruct-q4.gguf \
  cargo test --release --locked -p ferrite-server --all-features \
    --test real_models http_phi3:: -- --ignored --test-threads=1
```

Do not split every artifact-gated case into a separate integration binary.
That repeats code generation and linking for tests that are normally ignored.

When adding a dependency:

1. Explain why the standard library or an existing dependency is insufficient.
2. Review maintenance, MSRV, unsafe surface, transitive graph, license, and
   advisories.
3. Use an explicit compatible version, never `*`.
4. Run `cargo tree --duplicates`, `cargo machete`, `cargo audit`, and
   `cargo deny --all-features --locked check`.
5. Commit the updated lockfile for reproducible binaries and CI.

## Package checks

The publishable library crates use this release order:

```sh
cargo package -p ferrite-fixtures --locked
cargo publish -p ferrite-fixtures --locked
cargo package -p ferrite-model --locked
cargo publish -p ferrite-model --locked
cargo package -p ferrite-inference --locked
cargo publish -p ferrite-inference --locked
```

Cargo verifies registry dependencies while preparing a package. Therefore,
`ferrite-inference` cannot complete `cargo package` until the exact
`ferrite-fixtures` and `ferrite-model` versions in its manifest are available
from crates.io. Before the first library release, validate the inference archive
file set with:

```sh
cargo package -p ferrite-inference --locked --list
```

Do not publish from an unreviewed working tree. Package verification and the
documented release order are release gates, not substitutes for the full
validation sequence.
