# Ferrite documentation

This directory is the maintained documentation for users and contributors.
Start with the path that matches your goal.

## Run Ferrite

1. [Getting started](getting-started.md), build Ferrite and run a first model.
2. [Performance golden path](performance.md), use the fastest validated path
   for the current machine without sacrificing reproducibility.
3. [Command-line interface](cli.md), generate, stream, profile, and benchmark.
4. [HTTP server](server.md), configure and operate the local server.
5. [OpenAI API compatibility](openai-api.md), understand supported endpoints,
   request options, response behavior, and errors.
6. [Models and tensor formats](models.md), choose a compatible GGUF artifact.
7. [Current limitations](limitations.md), understand the alpha compatibility
   boundary before deployment.
8. [Troubleshooting](troubleshooting.md), diagnose common failures.

## Understand and contribute

- [Architecture](architecture.md)
- [Library API](library-api.md)
- [Operational tools](benchmark-tools.md)
- [Evaluation and regression gates](evaluation.md)
- [Development guide](development.md)
- [Safety policy](safety.md)
- [Release process](releasing.md)
- [Contributing guide](../CONTRIBUTING.md)
- [Security policy](../SECURITY.md)
- [Changelog](../CHANGELOG.md)

## Engineering evidence

- [`adr/`](adr/README.md) contains durable architecture decisions.
- [`benchmarks/`](benchmarks/README.md) contains curated methods and milestone
  results.
- [`engineering/rust-quality.md`](engineering/rust-quality.md) records the
  primary-source Rust quality baseline used by the repository.
- [`../scripts/evals/`](../scripts/evals/) contains machine-readable eval
  records that remain part of performance regression history.

Git history retains removed experiments and session notes when archaeology is
needed. The root README and this directory define the maintained contract.
