# Release process

Ferrite releases are evidence-backed and reproducible. Do not publish directly
from an unreviewed or dirty worktree.

## 1. Prepare

1. Update versions in every published package and internal dependency.
2. Update `Cargo.lock` with the pinned toolchain.
3. Move completed entries from `Unreleased` in the changelog to the release.
4. Confirm the declared Rust 1.96 MSRV still passes.
5. Record any behavior, API, safety, or performance change in maintained docs.

## 2. Validate

Run every command in [evaluation and regression gates](evaluation.md). Also run:

```sh
cargo semver-checks check-release -p ferrite-model
cargo semver-checks check-release -p ferrite-inference
cargo package -p ferrite-model --locked
cargo package -p ferrite-inference --locked --list
```

Inspect package contents with `cargo package --list`. No model, generated cache,
private plan, or unrelated benchmark asset may enter a crate archive.
Run a full `cargo package -p ferrite-inference --locked` once its exact
`ferrite-model` dependency version is available from the registry.

Performance-affecting releases also require a clean `scripts/eval.sh` artifact
with the model hash, token trace, build flags, machine, and regression analysis.

## 3. Publish in dependency order

```sh
cargo publish -p ferrite-model --locked
cargo publish -p ferrite-inference --locked
```

Wait until `ferrite-model` is available from the registry before packaging or
publishing `ferrite-inference`. The CLI, fixtures, and server packages set
`publish = false`.

## 4. Tag and verify

Create an annotated `vX.Y.Z` tag on the exact release commit, push it, and
verify that the tag, local branch, and remote branch resolve to the intended
commit. Confirm a clean worktree after publication.

## 5. Record

Publish release notes from the changelog, including breaking changes,
experimental status, supported models and platforms, performance evidence, and
known limitations.
