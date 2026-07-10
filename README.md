# Ferrite

Ferrite is a CPU-native large language model inference engine written in Rust.
It loads GGUF models, runs quantized inference with architecture-specific SIMD
kernels, and exposes both a command-line interface and an OpenAI-compatible
HTTP server.

Ferrite is currently alpha software. The supported surface is deliberately
small and well tested, but model coverage and sampling features are not yet
complete.

## Highlights

- Llama and Qwen2 model architectures, including Qwen2.5 GGUF artifacts.
- F32, F16, BF16, Q4_K, Q5_0, Q6_K, and Q8_0 tensor loading.
- NEON, Arm I8MM, and x86_64 AVX2 kernels with portable fallbacks.
- Greedy text generation, token streaming, profiling, and reproducible evals.
- OpenAI-compatible models, completions, and chat-completions endpoints.
- Optional prompt-prefix caching and experimental continuous batching.
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

Place a supported GGUF model under `target/models/`, then run a completion:

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
CI runs formatting, strict Clippy, default and all-feature tests on Linux and
macOS, plus a separate Rust 1.96 MSRV check. Linux jobs also enforce rustdoc,
doctests, documentation, repository hygiene, eval-harness tests, package
contents, RustSec advisories, licenses, duplicate dependencies, and sources.

## License

Ferrite is available under the [MIT License](LICENSE).
