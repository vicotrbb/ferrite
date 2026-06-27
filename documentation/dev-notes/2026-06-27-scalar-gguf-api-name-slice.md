# 2026-06-27 Scalar GGUF API Name Slice

## Scope

This slice adds `ScalarLlamaModel::from_gguf_scalar` as the accurate public
constructor for the scalar GGUF loader.

## Implementation

- Added `ScalarLlamaModel::from_gguf_scalar`.
- Renamed the internal loader entry point to `load_scalar`.
- Kept `from_gguf_f32` and `from_gguf_unquantized` as compatibility wrappers.
- Updated active CLI and scalar reference test call sites to the scalar name.

## Boundaries

This is a naming and API clarity slice. It does not add model compatibility,
new tensor formats, or new inference behavior.

## Evidence

- Red: `cargo test -p ferrite-inference --test scalar_reference
  loads_scalar_llama_reference_weights_from_f32_gguf_fixture` failed because
  `ScalarLlamaModel::from_gguf_scalar` did not exist.
- Green: the same targeted test passed after adding the scalar constructor.
