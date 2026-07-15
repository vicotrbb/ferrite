# Ferrite

Ferrite is a CPU-native large language model inference engine written in Rust.
It loads GGUF models, runs quantized inference with architecture-specific SIMD
kernels, and exposes both a command-line interface and an OpenAI-compatible
HTTP server.

Ferrite is currently alpha software. The supported surface is deliberately
small and well tested, but model, template, and sampling coverage are not yet
complete.

## Install

Ferrite's supported user installs are versioned release archives and the
official server image. Do not use `cargo install ferrite`: that crates.io name
belongs to an unrelated project.

Download the archive for your platform from the
[GitHub Releases page](https://github.com/vicotrbb/ferrite/releases), verify it
against the release's `SHA256SUMS` file, and then verify its provenance:

```sh
gh release verify-asset v<version> ferrite-v<version>-<target>.tar.gz \
  --repo vicotrbb/ferrite
gh attestation verify ferrite-v<version>-<target>.tar.gz \
  --repo vicotrbb/ferrite
```

The initial supported archive targets are macOS arm64 and statically linked
Linux x86_64. See [installation and verification](docs/install.md) for
extraction, container, and model-integrity instructions.

## Highlights

- Llama, Qwen2, and Phi-3 model architectures, including a verified official
  Phi-3 Mini 4K Instruct path.
- F32, F16, BF16, Q4_K, Q5_0, Q5_K, Q6_K, and Q8_0 tensor loading.
- Shared mapped storage for dense-16 and quantized matrices, with bounded
  token-embedding row decoding.
- Central runtime dispatch for NEON, Arm DotProd and I8MM, and x86_64 AVX2,
  with a forceable portable correctness provider.
- Fused greedy generation plus seeded temperature, top-k, top-p, min-p,
  penalty, and logit-bias sampling.
- OpenAI-compatible models, completions, chat completions, and a bounded
  non-streaming Responses endpoint.
- Grammar-constrained JSON objects and bounded function-call parsing. Ferrite
  reports function calls but never executes application tools.
- Unified prompt-prefix reuse and experimental continuous batching for greedy
  streaming and non-streaming requests, with bounded queues and cache budgets.
- Optional fixed-block Locus KV storage with explicit per-session capacity.
- No model or binary test assets committed to the repository. Test GGUF data
  is generated in memory, and real-model tests use local artifacts explicitly.

## Quick start

Install Rust 1.96.1 through [rustup](https://rustup.rs/), then build the release
binaries:

```sh
git clone https://github.com/vicotrbb/ferrite.git
cd ferrite
cargo build --release --locked -p ferrite-cli -p ferrite-server
```

For the shortest verified first run, ask Ferrite to acquire the pinned
Microsoft Phi-3 Mini 4K Instruct Q4 artifact and generate locally:

```sh
target/release/ferrite \
  --model-id phi3-mini-4k-instruct-q4 \
  --prompt '<|user|>
Write one sentence about Rust.<|end|>
<|assistant|>' \
  --generate-tokens 32 \
  --stream
```

The first invocation downloads about 2.39 GB over HTTPS, verifies the pinned
size and SHA-256, publishes it atomically into the user model cache, and makes
the artifact and provenance manifest read-only. Later invocations reverify the
cached bytes. Pass `--offline` to prohibit acquisition.

To use another supported GGUF, provide its path directly:

```sh
target/release/ferrite \
  --model target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --prompt 'Write one sentence about Rust.' \
  --generate-tokens 32 \
  --stream
```

Or start the HTTP server:

```sh
target/release/ferrite-server \
  --model target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --model-id qwen2.5-0.5b-q4_k_m \
  --bind 127.0.0.1:8080 \
  --api-key local-secret
```

```sh
curl http://127.0.0.1:8080/v1/chat/completions \
  -H 'authorization: Bearer local-secret' \
  -H 'content-type: application/json' \
  -d '{"model":"qwen2.5-0.5b-q4_k_m","messages":[{"role":"user","content":"Hello"}],"max_completion_tokens":32}'
```

For the measured, machine-specific fast path, follow the
[performance golden path](docs/performance.md). Do not assume that an
experimental kernel is faster or compatible on every CPU.

## Documentation

- [Documentation index](docs/README.md)
- [Getting started](docs/getting-started.md)
- [Performance golden path](docs/performance.md)
- [Command-line interface](docs/cli.md)
- [HTTP server](docs/server.md)
- [OpenAI API compatibility](docs/openai-api.md)
- [Models and tensor formats](docs/models.md)
- [Architecture](docs/architecture.md)
- [Library API](docs/library-api.md)
- [Operational tools](docs/benchmark-tools.md)
- [Acceptance matrix](docs/acceptance-matrix.md)
- [Evaluation and regression gates](docs/evaluation.md)
- [Development guide](docs/development.md)
- [Safety policy](docs/safety.md)
- [Current limitations](docs/limitations.md)
- [Troubleshooting](docs/troubleshooting.md)
- [Release process](docs/releasing.md)
- [Changelog](CHANGELOG.md)

Architecture decisions and curated benchmark evidence live beside the guides
under `docs/`. Raw reproducible eval records remain under `scripts/evals/`.
Transient plans, session notes, private tool state, and model binaries are not
repository artifacts.

## Repository layout

```text
crates/ferrite-model       GGUF parsing and tokenization
crates/ferrite-inference   model loading, sessions, cache, and CPU kernels
crates/ferrite-cli         local generation, profiling, and benchmarking
crates/ferrite-server      OpenAI-compatible serving and eval clients
crates/ferrite-fixtures    generated test fixtures
docs                       maintained user and contributor documentation
docs/adr                   durable architecture decisions
docs/benchmarks            curated benchmark protocols and milestone evidence
scripts                    eval harness, raw eval records, and repository checks
```

## Quality gates

The required local checks are documented in [CONTRIBUTING.md](CONTRIBUTING.md).
CI runs formatting, strict Clippy, and default plus all-feature tests on Linux
x86_64, Linux arm64, macOS arm64, macOS x86_64, and Windows x86_64, plus a
separate Rust 1.96 MSRV check and focused Rosetta parity. Linux jobs also
enforce rustdoc, doctests, documentation, repository hygiene, eval-harness
tests, package contents, RustSec advisories, licenses, duplicate dependencies,
and sources.

## License

Ferrite is available under the [Apache License 2.0](LICENSE).
