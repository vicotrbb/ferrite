# 2026-06-27 Q6_K Loader Slice

## Scope

This slice adds scalar dequantization support for `GGML_TYPE_Q6_K` tensors.

## Implementation

- Added scalar Q6_K dequantization in `ferrite-inference`.
- Added a tensor unit test for signed Q6_K block reconstruction.
- Reused the downloaded Tier 0 SmolLM2 Q4_K_M GGUF probe as the model-level
  compatibility gate.

## Boundaries

This proves Ferrite can load and run the downloaded Tier 0 SmolLM2 Q4_K_M GGUF
locally with token-id prompts. It does not yet prove output parity against
`llama.cpp`, because no local `llama-cli`, `llama.cpp`, or `ollama` binary was
available during this slice.

## Model Probe

- Repo: `bartowski/SmolLM2-135M-Instruct-GGUF`
- File: `SmolLM2-135M-Instruct-Q4_K_M.gguf`
- Hugging Face repo commit observed during download: `09816acd5d99df7be770d85ea30822623dab342c`
- Downloaded local path: `target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf`
- Observed local size: 101 MB

## Evidence

- Real-model red before this slice:
  `cargo run -p ferrite-cli -- --model
  target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf --prompt-token-ids 1`
  failed on `blk.0.ffn_down.weight` with tensor type `Q6K`.
- Test red: `cargo test -p ferrite-inference q6_k` failed because
  `q6_k_values_from_le_bytes` did not exist.
- Test green: the same targeted test passed after adding Q6_K dequantization.
- Real-model green: the same CLI probe succeeded with:
  - `prompt_token_ids=1`
  - `next_token_id=28`
  - `next_token=,`
