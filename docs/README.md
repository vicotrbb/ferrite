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
7. [Troubleshooting](troubleshooting.md), diagnose common failures.

## Understand and contribute

- [Architecture](architecture.md)
- [Evaluation and regression gates](evaluation.md)
- [Development guide](development.md)
- [Safety policy](safety.md)
- [Contributing guide](../CONTRIBUTING.md)
- [Security policy](../SECURITY.md)

## Engineering evidence

The [`documentation/`](../documentation/README.md) tree is the evidence record:

- `adr/` contains durable architecture decisions.
- `benchmarks/` contains benchmark methods and results.
- `dev-notes/` contains historical implementation evidence.
- `research/` contains focused follow-up research.
- `theories/` contains hypotheses that are not implementation guarantees.

Historical notes can describe superseded behavior. The root README and this
directory define the maintained user-facing contract.
