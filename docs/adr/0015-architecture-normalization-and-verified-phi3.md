# ADR 0015: Architecture normalization and verified Phi-3 acquisition

Date: 2026-07-13

Status: Accepted

## Context

Ferrite's execution model was correct for Llama and Qwen2, but its public and
internal names still implied that every model had Llama tensor storage. Adding
another family directly inside decode would spread architecture conditions
through attention, feed-forward, RoPE, and cache logic. It would also make
model-filename exceptions tempting.

A useful broader-model proof needed a publisher-owned GGUF artifact with clear
provenance and a size suitable for CPU use. Downloading mutable latest files or
trusting a cache entry by filename would not meet Ferrite's local supply-chain
boundary.

## Decision

`ferrite-model` exposes a stable `ModelArchitecture`, `ModelConfig`, and
`ArchitectureExecution` descriptor. The descriptor records attention
projection layout, feed-forward projection layout, and rotary pairing. Loader
adapters normalize architecture-specific tensors into common transformer
weights before execution. Existing Llama-named public types remain available
for source compatibility, with architecture-neutral aliases for new code.

Ferrite adds the `phi3` GGUF architecture. Its adapter splits fused QKV and
gate-up matrices by validated row ranges and uses split-half rotary pairing.
The shared runtime adds validated Q5_K storage and a portable reference matvec.
No optimized Q5_K kernel is claimed. The tokenizer adds scored SentencePiece
merging, byte fallback, and Phi-3 special-token boundary behavior compatible
with the artifact metadata.

The CLI built-in registry pins one official Microsoft artifact by source,
revision, license, filename, exact byte size, and SHA-256. Acquisition is
explicit through `--model-id`, resumable through a partial file, restricted to
HTTPS, and published by atomic rename only after size and hash verification.
The final model and JSON provenance manifest are read-only. Cache hits are
rehashed before use, symbolic links are rejected, and `--offline` prohibits a
cache miss from reaching the network.

The initial registry entry is:

- ID: `phi3-mini-4k-instruct-q4`
- source: `https://huggingface.co/microsoft/Phi-3-mini-4k-instruct-gguf`
- revision: `a64113399c2f6b8ad3e11c394733a2ddadaa7f33`
- license: MIT
- filename: `Phi-3-mini-4k-instruct-q4.gguf`
- size: 2,393,231,072 bytes
- SHA-256: `8a83c7fb9049a9b2e92266fa7ad04933bb53aa1e85136b7b30f1b8000ff2edef`

## Consequences

Architecture-specific storage no longer changes the decode contract. A new
family must provide a bounded metadata and tensor adapter, tokenizer evidence,
fixture coverage, and real-artifact parity before it is listed as supported.

The first-run command can acquire and run one useful 3.8B model without manual
checksum handling. Network access remains opt-in and limited to model bytes.
Ferrite has no prompt telemetry or hosted inference fallback.

The server still requires an explicit model path. This avoids giving a network
service implicit download authority. Operators can pre-acquire the built-in
artifact with the CLI, then mount its verified cache path read-only.

Q5_K correctness is available, but its portable implementation can be slower
than optimized formats. No throughput, TTFT, or RSS improvement follows from
this decision.

## Alternatives Considered

- Add Phi-3 conditions throughout the Llama session. Rejected because tensor
  layout is a loader concern and decode should consume normalized weights.
- Select behavior from model filenames. Rejected because filenames are mutable
  and do not define GGUF semantics.
- Download an unpinned model from a moving branch. Rejected because provenance
  and reproducibility require an immutable revision, exact size, and hash.
- Trust a previously cached filename. Rejected because replacement or partial
  content would bypass the supply-chain check.
- Add an ISA-specific Q5_K kernel with the format. Rejected until profiling and
  clean repeated end-to-end evaluation justify the maintenance and unsafe-code
  boundary.

## Evidence

- `crates/ferrite-model/src/gguf_config.rs` defines architecture and execution
  layouts.
- `crates/ferrite-inference/src/scalar/loader.rs` normalizes fused Phi-3
  tensors.
- `crates/ferrite-inference/src/scalar/q5_k.rs` contains the portable Q5_K
  implementation and block tests.
- `crates/ferrite-model/src/tokenizer/spm.rs` contains SentencePiece merging and
  byte fallback.
- `crates/ferrite-cli/src/model_acquisition.rs` owns the pinned registry and
  verified cache transaction.
- The official
  [Microsoft Phi-3 Mini GGUF model card](https://huggingface.co/microsoft/Phi-3-mini-4k-instruct-gguf)
  identifies the 3.8B model, Q4_K_M artifact, chat format, and MIT license.
- For the same 12 rendered prompt token IDs, pinned llama.cpp revision
  `6eddde06a4f25d55d538b5d15628dcc2b6882147` and Ferrite portable plus auto
  providers selected token ID 7521. All three ordered their top-five token IDs
  as 7521, 426, 8853, 421, 23230. This is one-token correctness evidence, not a
  performance result.
- `crates/ferrite-server/tests/openai_real_phi3.rs` rehashes the pinned model
  and checks deterministic non-streaming and streaming HTTP generation. It
  proves that Phi-3 token 32007 (`<|end|>`) ends generation, contributes to
  usage accounting, and remains absent from visible response text. Ferrite and
  the pinned llama.cpp reference share the first two generated IDs for this
  prompt but have different complete traces, so this evidence does not claim
  multi-token reference parity or performance.
