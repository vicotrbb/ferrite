# Installation and verification

Ferrite ships versioned native archives and an official OCI server image. The
release archive contains `ferrite`, `ferrite-server`, the Apache License 2.0,
the changelog, and the security policy. Model weights are intentionally not
bundled with either distribution.

## Platform paths

| Host | Recommended path | Current evidence boundary |
| --- | --- | --- |
| macOS arm64 | Verified release archive | Native CI and release build |
| Linux x86_64 | Verified static release archive or OCI image | Native CI and release build |
| Linux arm64 | Multi-architecture OCI image or source build | Native CI; no native archive |
| macOS x86_64 | Source build | Native Intel CI and Rosetta parity job; no native archive |
| Windows x86_64 | Source build | Native CI; no native archive or retained real-model performance result |

Ferrite selects CPU features at runtime and has a forceable portable provider,
but a successful build is not a performance claim. Use the evaluation harness
on the deployment host before choosing threads or experimental policies.

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

## Build from source on macOS or Linux

Install Rust through the official
[rustup installer](https://rust-lang.org/install.html). macOS also needs the
Xcode command-line tools, and Linux needs a C compiler and linker. Then:

```sh
git clone https://github.com/vicotrbb/ferrite.git
cd ferrite
cargo build --release --locked -p ferrite-cli -p ferrite-server
mkdir -p ~/.local/bin
install -m 0755 target/release/ferrite ~/.local/bin/ferrite
install -m 0755 target/release/ferrite-server ~/.local/bin/ferrite-server
ferrite --version
ferrite-server --version
```

This is the supported route for Linux arm64 and macOS x86_64 until those
targets have versioned native archives.

## Build from source on Windows x86_64

Use the official
[Rust Windows installer](https://rust-lang.org/install.html) and accept the
prompt to install the Visual Studio C++ Build Tools when they are absent. Open
a new PowerShell session after installation, then run:

```powershell
git clone https://github.com/vicotrbb/ferrite.git
Set-Location ferrite
cargo build --release --locked -p ferrite-cli -p ferrite-server
New-Item -ItemType Directory -Force "$HOME\bin"
Copy-Item target\release\ferrite.exe "$HOME\bin\ferrite.exe"
Copy-Item target\release\ferrite-server.exe "$HOME\bin\ferrite-server.exe"
& "$HOME\bin\ferrite.exe" --version
& "$HOME\bin\ferrite-server.exe" --version
```

Add `$HOME\bin` to the user `PATH` if you want to invoke the binaries without
their full paths. Windows currently has correctness CI, but not a signed
release archive or a retained native performance result.

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

For the built-in `phi3-mini-4k-instruct-q4` registry entry, the CLI performs
the pinned size and SHA-256 verification automatically, writes a provenance
manifest, and makes the completed cache read-only. See
[models and tensor formats](models.md) for the exact artifact record.
