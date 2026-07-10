# Models and tensor formats

Ferrite intentionally supports a narrow GGUF surface. Verify compatibility
before downloading or deploying a large artifact.

## Supported model architectures

- `llama`
- `qwen2`, including tested Qwen2.5 artifacts

Architecture is read from `general.architecture`. Other architectures are
rejected during model configuration instead of being interpreted as Llama.

## GGUF and tensor support

Ferrite currently requires GGUF version 3.

The inference loader supports these tensor encodings:

- F32
- F16
- BF16
- Q4_K
- Q5_0
- Q6_K
- Q8_0

The parser recognizes additional GGML type identifiers so it can report the
file accurately, but recognition does not mean that inference is implemented.
Unsupported tensor types fail with an explicit error.

Optimized retained matrix storage exists for Q4_K, Q5_0, Q6_K, and Q8_0.
Dense tensor encodings are converted to finite F32 values during loading.

## Required model shape

The loader expects a transformer layout with validated context, embedding,
block, feed-forward, head, KV-head, key, value, RoPE, and RMS-normalization
metadata. It validates tensor ranks, dimensions, byte ranges, alignments,
duplicate names, finite scales, and compatible grouped-query attention.

Ferrite supports tied output weights when `output.weight` is absent and
`token_embd.weight` has the required output shape.

## Tokenizer support

Ferrite reads the GGUF token vocabulary, token types, merge rules, and optional
EOS token ID. It supports atomic longest-prefix encoding when no merge table is
present, and byte-aware BPE when valid merge metadata is present.

Chat prompt rendering is Ferrite's current local template, not an arbitrary
model-provided Jinja template. Token parity depends on using a model whose
metadata and expected prompt format match the tested surface.

## Choosing a model

For initial validation, use the Qwen2.5 0.5B Instruct Q4_K_M reference artifact
used by `scripts/eval.sh`. It is small enough for rapid regression runs and
exercises the optimized quantized path.

Before using another artifact:

1. Confirm GGUF version 3.
2. Confirm `llama` or `qwen2` architecture metadata.
3. Confirm every required weight uses a supported tensor type.
4. Run one-token CLI inference and inspect errors.
5. Run a deterministic token trace against a trusted reference.
6. Run the complete eval harness before making performance claims.

## Model files and licensing

Model binaries are not committed to Ferrite. Store local artifacts under
`target/models/` or provide an explicit path. Review the model license,
training terms, redistribution rights, and prompt-data policy separately from
Ferrite's Apache License 2.0.
