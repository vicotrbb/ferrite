# Release process

Ferrite releases are evidence-backed, immutable, and tag-driven. Do not
publish from a dirty worktree or move a release tag after it is pushed.

## Release contract

Every final `vX.Y.Z` tag is expected to produce:

- verified native archives for macOS arm64 and statically linked Linux x86_64;
- `SHA256SUMS`, SPDX SBOMs, and GitHub build attestations for those archives;
- a non-root multi-architecture `ghcr.io/vicotrbb/ferrite` server image with
  OCI provenance and SBOM attestations;
- `ferrite-fixtures`, `ferrite-model`, and `ferrite-inference` on crates.io;
- immutable GitHub Release assets and notes extracted from `CHANGELOG.md`.

The `ferrite` and `ferrite-cli` crates.io names are owned by unrelated projects.
Native archives are therefore the supported installation route for Ferrite's
command-line binaries. The published Rust surface is limited to the two runtime
library crates above. `ferrite-fixtures` is published only so
`ferrite-inference` can verify its source-distributed integration tests; it is
not a supported production API.

## 1. Prepare a release commit

1. Set the same release version in all workspace packages and internal
   dependency requirements.
2. Update `Cargo.lock` with the pinned toolchain.
3. Move completed entries from `Unreleased` into `## X.Y.Z - YYYY-MM-DD` in the
   changelog.
4. Record behavior, API, safety, and performance changes in maintained docs.
5. Run the structural gate:

   ```sh
   python3 scripts/release_preflight.py --version X.Y.Z --require-clean
   ```

The release commit must be merged through the protected `main` branch before a
tag is created.

## 2. Validate

Run every command in [evaluation and regression gates](evaluation.md). Also run:

```sh
cargo package -p ferrite-fixtures --locked
cargo package -p ferrite-model --locked
cargo package -p ferrite-inference --locked --list
python3 scripts/release_tools_test.py
```

For a release after the initial library publication, also run:

```sh
cargo semver-checks check-release -p ferrite-fixtures
cargo semver-checks check-release -p ferrite-model
cargo semver-checks check-release -p ferrite-inference
```

The initial `0.1.0` publication has no registry baseline, so those
semantic-version checks are intentionally skipped for that one release. The tag
workflow runs them automatically for later library versions.

Inspect package contents with `cargo package --list`. No model, generated cache,
private plan, or unrelated benchmark asset may enter a crate archive.
Run a full `cargo package -p ferrite-inference --locked` once the exact
`ferrite-fixtures` and `ferrite-model` dependency versions are available from
the registry.

Performance-affecting releases also require a clean `scripts/eval.sh` artifact
with the model hash, token trace, build flags, machine, and regression analysis.

## 3. Publish with one immutable tag

From the merged release commit, create and push an annotated tag:

```sh
git tag -a vX.Y.Z -m "Ferrite vX.Y.Z"
git push origin vX.Y.Z
```

The `release.yml` workflow verifies that the tag names the current `main`
commit, runs the release checks, builds artifacts, publishes the image and
library crates, attaches checksums and SBOMs to a draft release, generates
attestations, then publishes the immutable GitHub Release. It is the only
supported publication path after the initial crates.io bootstrap.

The first publication of all three crates.io packages must be performed manually
from the exact release commit, in fixture, model, then inference order. Configure
crates.io trusted publishing immediately afterward for each package, using
repository `vicotrbb/ferrite`, workflow
`.github/workflows/release.yml`, and environment `release`. Subsequent tags use
short-lived OIDC credentials and do not require a stored registry token.

## 4. Verify as a consumer

After the workflow succeeds, verify the immutable release and a downloaded
asset:

```sh
gh release verify vX.Y.Z --repo vicotrbb/ferrite
gh release verify-asset vX.Y.Z ferrite-vX.Y.Z-<target>.tar.gz \
  --repo vicotrbb/ferrite
gh attestation verify ferrite-vX.Y.Z-<target>.tar.gz \
  --repo vicotrbb/ferrite
gh attestation verify oci://ghcr.io/vicotrbb/ferrite:vX.Y.Z \
  --repo vicotrbb/ferrite
```

Run each release binary's `--version` command. For the server image, run a
localhost health and OpenAI-compatible request against a mounted, checksum
verified GGUF model. Record results in the release notes or benchmark evidence.

## 5. Correcting a published release

Never replace an archive, image digest, or release tag. For an ordinary defect,
publish a new patch release. For a compromised or vulnerable library release,
publish a security advisory, revoke affected credentials, and yank the affected
crates.io version when appropriate. Existing immutable assets remain available
for auditability.
