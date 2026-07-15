# Models and tensor formats

Ferrite intentionally supports a narrow GGUF surface. Verify compatibility
before downloading or deploying a large artifact.

## Supported model architectures

- `llama`
- `qwen2`, including tested Qwen2.5 artifacts
- `phi3`, including the official Phi-3 Mini 4K Instruct Q4 artifact

Architecture is read from `general.architecture`. Other architectures are
rejected during model configuration instead of being interpreted as Llama.

## GGUF and tensor support

Ferrite currently requires GGUF version 3.

The parser follows the upstream
[GGUF v3 format](https://github.com/ggml-org/ggml/blob/master/docs/gguf.md),
including nested metadata arrays, but applies explicit resource limits before
allocating from untrusted header counts. A file may contain at most 65,536
metadata entries, 65,536 tensors, 1,048,576 decoded metadata values, and 64
nested metadata arrays. One metadata string is limited to 16 MiB and all
decoded strings together are limited to 256 MiB. Metadata keys retain the GGUF
65,535-byte limit and tensor names are limited to 64 bytes. Capacity overflow,
impossible array lengths, excessive nesting, and fallible reservation failures
return parser errors.

The inference loader supports these tensor encodings:

- F32
- F16
- BF16
- Q4_K
- Q5_0
- Q5_K
- Q6_K
- Q8_0

The parser recognizes additional GGML type identifiers so it can report the
file accurately, but recognition does not mean that inference is implemented.
Unsupported tensor types fail with an explicit error.

F16, BF16, and every supported quantized matrix encoding retain validated byte
ranges from one shared read-only GGUF mapping. F16 and BF16 matvecs convert
lanes during accumulation instead of expanding the complete matrix to F32.
F32 matrices and required vectors use owned finite values. Q5_K uses a
validated architecture-neutral reference matvec.

Token embedding lookup for Q4_K, Q5_K, and Q6_K decodes only the minimal block
window that intersects the selected row. It does not materialize an F32 copy of
the complete embedding matrix.

## Required model shape

The loader expects a transformer layout with validated context, embedding,
block, feed-forward, head, KV-head, key, value, RoPE, and RMS-normalization
metadata. It validates tensor ranks, dimensions, byte ranges, alignments,
duplicate names, finite scales, and compatible grouped-query attention.

Ferrite supports tied output weights when `output.weight` is absent and
`token_embd.weight` has the required output shape.

## Tokenizer support

Ferrite reads the GGUF token vocabulary, token types, scores, merge rules,
optional BOS, EOS, EOT, and EOM token IDs, and configured boundary-insertion
flags. It supports atomic longest-prefix encoding when no merge metadata is
present, byte-aware BPE when valid merge rules are present, and scored
SentencePiece merging with byte fallback for Llama-tokenizer artifacts such as
Phi-3. Control and user-defined special-token strings remain atomic.

Generation stops on explicit EOS, EOT, and EOM metadata plus the bounded turn
terminators used by the supported template families: Llama `</s>` and
`<|eot_id|>`, Qwen `<|im_end|>`, and Phi-3 `<|end|>`. Inferred terminators must
be control or user-defined tokens, so ordinary text with the same spelling
does not acquire stop behavior.

Chat prompt rendering reads `tokenizer.chat_template` and recognizes bounded
Qwen-style ChatML, Llama 3 header, Llama 2 instruction, and Phi-3 turn
families. Ferrite does not execute arbitrary Jinja. Missing or unrecognized
templates use an explicit role-labelled fallback. Token parity depends on
using a model whose metadata and expected prompt format match this tested
surface.

## Architecture normalization

GGUF parsing exposes a stable architecture descriptor for attention layout,
feed-forward layout, and rotary pairing. Loader adapters split Phi-3 fused QKV
and gate-up tensors into the same execution-facing weights used by the common
transformer runtime. This keeps architecture-specific metadata and storage out
of decode logic and prevents model-filename-specific behavior.

## Verified built-in model

The CLI registry currently contains one artifact:

| Field | Value |
| --- | --- |
| ID | `phi3-mini-4k-instruct-q4` |
| Source | `https://huggingface.co/microsoft/Phi-3-mini-4k-instruct-gguf` |
| Revision | `a64113399c2f6b8ad3e11c394733a2ddadaa7f33` |
| License | MIT |
| Filename | `Phi-3-mini-4k-instruct-q4.gguf` |
| Size | 2,393,231,072 bytes |
| SHA-256 | `8a83c7fb9049a9b2e92266fa7ad04933bb53aa1e85136b7b30f1b8000ff2edef` |

Run it with `ferrite --model-id phi3-mini-4k-instruct-q4 ...`. The registry
entry records provenance; it does not transfer or reinterpret the model's MIT
license. Review the official
[Microsoft model card](https://huggingface.co/microsoft/Phi-3-mini-4k-instruct-gguf)
before use.

## Choosing a model

For initial validation, use the Qwen2.5 0.5B Instruct Q4_K_M reference artifact
used by `scripts/eval.sh`. It is small enough for rapid regression runs and
exercises the optimized quantized path.

Before using another artifact:

1. Confirm GGUF version 3.
2. Confirm `llama`, `qwen2`, or `phi3` architecture metadata.
3. Confirm every required weight uses a supported tensor type.
4. Run one-token CLI inference and inspect errors.
5. Run a deterministic token trace against a trusted reference.
6. Run the complete eval harness before making performance claims.

## Model files and licensing

Model binaries are not committed to Ferrite. Store local artifacts under
`target/models/` or provide an explicit path. Review the model license,
training terms, redistribution rights, and prompt-data policy separately from
Ferrite's Apache License 2.0.
