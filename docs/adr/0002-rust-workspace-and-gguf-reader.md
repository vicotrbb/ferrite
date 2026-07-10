# ADR 0002: Rust Workspace and GGUF Reader Boundary

Date: 2026-06-27

Status: Accepted

## Context

Ferrite's first milestone requires loading a Tier 0 Llama-family GGUF model,
parsing metadata and tensors, and using those tensors in a scalar reference
forward path. The repository previously contained only documentation and
research, so there was no Rust workspace, crate boundary, parser API, or local
test harness.

The current GGUF specification confirms that GGUF v3 stores a header,
typed metadata, tensor info records, alignment padding, and tensor data. Tensor
offsets are relative to the tensor-data section and must respect
`general.alignment`, defaulting to 32 when omitted.

## Decision

Ferrite starts with a Cargo workspace and a `ferrite-model` library crate.
The crate owns model-format parsing and model-configuration extraction.

The initial durable boundary is:

- `ferrite_model::gguf::parse_gguf(&[u8]) -> Result<GgufFile, GgufError>`
- `GgufFile::tensor(name)` for tensor lookup by standardized GGUF name.
- `GgufFile::llama_config()` for extracting the Llama metadata required by
  Tier 0 scalar forward work.

The reader is safe Rust only. It validates magic, version, metadata keys,
metadata value types, alignment, tensor offsets, tensor byte ranges, UTF-8
strings, boolean encodings, and supported tensor storage sizes before exposing
tensor data ranges.

## Consequences

The first implementation boundary is intentionally smaller than full model
execution. It makes the next slice concrete: build scalar tensor views and a
minimal Llama forward path on top of validated GGUF tensor ranges.

Unsupported GGML tensor types fail explicitly when byte sizing is not yet
implemented. This is preferable to guessing layout for quantization formats
that require exact block sizes before mmap or casting can be safe.

## Alternatives Considered

Use an existing GGUF crate.

This was rejected for the initial core boundary because Ferrite needs direct
control over tensor alignment, byte ranges, and future mmap behavior. External
GGUF readers remain useful as references.

Start with a CLI or HTTP runtime crate.

This was rejected because the first milestone depends on model loading and
scalar correctness before serving concerns matter.

## Evidence

- `cargo test -p ferrite-model` first failed because `gguf` did not exist,
  establishing the red TDD step.
- After implementing the reader, `cargo test -p ferrite-model` passed 4 GGUF
  parser and Llama-config tests.
- The implementation uses no `unsafe` code and the workspace forbids unsafe
  code through lints.
