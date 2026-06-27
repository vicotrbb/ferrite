# 2026-06-27 GGUF Format Refresh

## Question

Which GGUF structural details are required before Ferrite can safely expose
tensor byte ranges for Tier 0 loading?

## Current Evidence

The current ggml GGUF specification describes GGUF v3 as:

- `GGUF` magic bytes.
- Version `3`.
- `u64` tensor count and metadata key-value count.
- Typed metadata values.
- Tensor info records with name, dimensions, GGML tensor type, and tensor-data
  relative offset.
- Global alignment controlled by `general.alignment`, defaulting to `32` when
  absent.
- Tensor-data padding after tensor info records to the next alignment boundary.

Tensor offsets are relative to the start of the tensor-data section, not the
start of the file, and must be aligned to the global alignment.

## Impact on Ferrite

Ferrite's parser should expose absolute byte ranges only after validating the
relative tensor offsets and computing the aligned tensor-data start. The parser
should not expose raw offsets as if they were file offsets.

The baseline quantization note listed Q4_K_M as 148 bytes per 256 values.
Current GGML block sizing for `GGML_TYPE_Q4_K` is 144 bytes per 256 values; the
medium/small distinction is encoded by file-type and tensor-type mixtures, not
by a separate GGML tensor type named Q4_K_M. Ferrite should treat exact GGML
block sizes as implementation facts that must be validated against current
upstream code or fixture evidence before dequantization work.

## Decision for Current Slice

The first reader implements byte sizing for common GGML tensor types and fails
explicitly for tensor types whose block layouts have not yet been encoded. This
keeps the parser useful for standard Tier 0 metadata and tensor range discovery
without silently guessing unsupported quantization layouts.

## Follow-Up

Before implementing additional quantization families, verify block layouts
against the current upstream `ggml` definitions and add fixture tests that prove
storage byte calculations for representative tensor shapes.

## 2026-06-27 Q4_K Verification Update

Ferrite refreshed the `Q4_K` layout against upstream `ggml-org/llama.cpp`
source before implementing scalar dequantization:

- `GGML_TYPE_Q4_K` uses one 144-byte block per 256 values.
- The block stores `d` and `dmin` as F16 values.
- The block stores 12 packed scale/min bytes for eight 32-value subblocks.
- The block stores 128 bytes of packed 4-bit quantized values.
- Dequantization applies each subblock as `d * scale * quant - dmin * min`.

Reference source:
`https://github.com/ggml-org/llama.cpp/blob/master/ggml/src/ggml-quants.c`

## 2026-06-27 Q5_0 Verification Update

Ferrite refreshed the `Q5_0` layout against upstream `ggml-org/llama.cpp`
source after a real SmolLM2 Q4_K_M probe failed on a `Q5_0` tensor:

- `GGML_TYPE_Q5_0` uses one 22-byte block per 32 values.
- The block stores one F16 scale value.
- The block stores four bytes of high bits.
- The block stores 16 bytes of packed low nibbles.
- Dequantization reconstructs signed quantized values in `[-16, 15]` and
  multiplies them by the F16 scale.

Reference source:
`https://github.com/ggml-org/llama.cpp/blob/master/ggml/src/ggml-quants.c`

## 2026-06-27 Q6_K Verification Update

Ferrite refreshed the `Q6_K` layout against upstream `ggml-org/llama.cpp`
source after the real SmolLM2 Q4_K_M probe advanced to a `Q6K` tensor:

- `GGML_TYPE_Q6_K` uses one 210-byte block per 256 values.
- The block stores 128 bytes of low 4-bit values.
- The block stores 64 bytes of upper 2-bit values.
- The block stores 16 signed 8-bit scales.
- The block stores one F16 super-block scale.
- Dequantization reconstructs signed quantized values in `[-32, 31]`, applies
  the per-group signed scale, and then applies the F16 super-block scale.

Reference source:
`https://github.com/ggml-org/llama.cpp/blob/master/ggml/src/ggml-quants.c`
