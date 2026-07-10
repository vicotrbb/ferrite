# Installation and verification

Ferrite ships versioned native archives and an official OCI server image. The
release archive contains `ferrite`, `ferrite-server`, the Apache License 2.0,
the changelog, and the security policy. Model weights are intentionally not
bundled with either distribution.

## Native archive

The initial release targets are:

| Target | Archive suffix |
| --- | --- |
| macOS arm64 | `aarch64-apple-darwin` |
| Linux x86_64, statically linked | `x86_64-unknown-linux-musl` |

Download the archive and its matching `SHA256SUMS` file from the GitHub Release
page into the same directory. From that directory, verify both the release
asset and its build provenance before extracting:

On macOS:

```sh
shasum -a 256 -c SHA256SUMS
```

On Linux:

```sh
sha256sum -c SHA256SUMS
```

```sh
gh release verify-asset v<version> ferrite-v<version>-<target>.tar.gz \
  --repo vicotrbb/ferrite
gh attestation verify ferrite-v<version>-<target>.tar.gz \
  --repo vicotrbb/ferrite
```

Then install the two public binaries on your `PATH`:

```sh
tar -xzf ferrite-v<version>-<target>.tar.gz
install -m 0755 ferrite-v<version>-<target>/bin/ferrite ~/.local/bin/ferrite
install -m 0755 ferrite-v<version>-<target>/bin/ferrite-server \
  ~/.local/bin/ferrite-server
ferrite --version
ferrite-server --version
```

The macOS `install` command accepts the same mode flag. Choose another
user-owned `PATH` directory if `~/.local/bin` is not configured on your host.

## OCI server image

The official image is `ghcr.io/vicotrbb/ferrite`. Prefer a version tag while
evaluating a release and a digest pin in an unattended deployment:

```sh
docker pull ghcr.io/vicotrbb/ferrite:v<version>
gh attestation verify oci://ghcr.io/vicotrbb/ferrite:v<version> \
  --repo vicotrbb/ferrite
```

Run the image without elevated privileges, mounting a model read-only. The
server defaults to loopback, so a container deployment must explicitly choose
its non-loopback bind address and authentication key:

```sh
docker run --rm --read-only --cap-drop=ALL \
  -p 8080:8080 \
  -v "$PWD/models:/models:ro" \
  ghcr.io/vicotrbb/ferrite:v<version> \
  --model /models/model.gguf \
  --model-id ferrite-local \
  --bind 0.0.0.0:8080 \
  --api-key "$(openssl rand -hex 32)"
```

For a persistent deployment, provision the key through the platform's secret
store rather than shell history. Keep TLS, request-size limits, access logs,
and network policy in a trusted reverse proxy as described in the
[server guide](server.md).

## Model integrity

The Ferrite executable and a GGUF model are separate supply chains. Download
models from their publisher, record the source URL, revision, license, and
SHA-256 value, and verify the checksum before mounting or opening the file:

```sh
shasum -a 256 model.gguf
```

Ferrite validates malformed model data defensively, but a successful parse does
not prove that a model has the expected origin, license, or behavior. Keep the
model provenance record with your deployment configuration.
