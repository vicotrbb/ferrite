# 2026-06-27 GGUF Reader Slice

## Slice

Establish the first Rust implementation slice for Ferrite's Tier 0 milestone:
a safe GGUF v3 reader that can parse model metadata, tensor info, tensor byte
ranges, and a Llama configuration from an in-memory GGUF byte fixture.

## Baseline Read

- `documentation/engineering/ferrite-goal-prompt.md`
- `documentation/engineering/ferrite-operating-model.md`
- `documentation/adr/0001-documentation-and-iteration-model.md`
- `research/11-testing-model-registry.md`
- `research/03-quantization-pipeline.md`
- `research/05-inference-engine-architecture.md`
- `research/06-model-architecture-compatibility.md`
- Current upstream GGUF specification from ggml.

## Implementation

- Added a Cargo workspace rooted at `Cargo.toml`.
- Added `crates/ferrite-model` as the first model-loading crate.
- Added `ferrite_model::gguf` with:
  - GGUF v3 header validation.
  - Typed metadata parsing.
  - Metadata-key validation.
  - `general.alignment` handling with the GGUF default of 32.
  - Tensor-info parsing and aligned tensor data range calculation.
  - Storage sizing for common GGML tensor types needed by GGUF model loading.
  - Llama metadata extraction for the Tier 0 scalar path.

## Validation

TDD red step:

```text
cargo test -p ferrite-model
error[E0583]: file not found for module `gguf`
```

Green step:

```text
cargo test -p ferrite-model
test result: ok. 4 passed; 0 failed
```

## Remaining Unproven

- No real Tier 0 model artifact has been downloaded or parsed yet.
- No tensor view, dequantization path, tokenizer, scalar forward path, or
  deterministic next-token comparison exists yet.
- No mmap boundary exists yet.
- Benchmarking is not relevant to this parser-only slice.

## Next Slice

Build scalar tensor views for F32/F16 and the minimal Llama reference operations
needed by a synthetic single-layer fixture: RMSNorm, matrix-vector multiply,
SwiGLU, logits, and deterministic argmax sampling. Then connect those pieces to
GGUF tensor names before testing against a real Tier 0 GGUF model.
