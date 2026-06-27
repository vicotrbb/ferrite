# 2026-06-27 Q5_0 Loader Slice

## Scope

This slice adds scalar dequantization support for `GGML_TYPE_Q5_0` tensors.

## Implementation

- Added Q5_0 fixture packing for generated GGUF fixtures.
- Added `scalar_llama_q5_0_gguf_fixture`.
- Added a scalar loader test for Q5_0 tensors.
- Added a tensor unit test for signed Q5_0 block reconstruction.
- Added scalar Q5_0 dequantization in `ferrite-inference`.

## Boundaries

This does not complete real Tier 0 model loading. It only covers Q5_0 block
dequantization. The downloaded SmolLM2 Q4_K_M probe now advances to the next
unsupported tensor type, `Q6K`.

## Evidence

- Real-model red before this slice:
  `cargo run -p ferrite-cli -- --model
  target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf --prompt-token-ids 1`
  failed on `blk.0.attn_q.weight` with tensor type `Q5_0`.
- Test red: `cargo test -p ferrite-inference --test scalar_reference
  loads_scalar_llama_reference_weights_from_q5_0_gguf_fixture` failed because
  Q5_0 was not supported by the scalar decoder.
- Test green: the same targeted test passed after adding Q5_0 dequantization.
- Signed-layout green: `cargo test -p ferrite-inference q5_0` passed the
  scalar fixture test and the signed Q5_0 decoder unit test.
- Real-model follow-up: the same CLI probe advanced to
  `blk.0.ffn_down.weight` and now fails on unsupported tensor type `Q6K`.
