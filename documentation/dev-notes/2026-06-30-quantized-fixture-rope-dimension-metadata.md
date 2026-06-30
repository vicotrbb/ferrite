# Quantized Fixture RoPE Dimension Metadata

The quantized scalar Llama GGUF fixtures now write valid
`llama.rope.dimension_count` metadata.

Recent GGUF model-config hardening correctly rejects explicit zero RoPE
dimension counts. The F32 and chat fixtures already used a valid value of `2`,
but the Q8_0, Q5_0, Q4_K, and Q6_K scalar fixtures still wrote `0`. Full
`ferrite-inference` verification exposed the mismatch when those fixtures began
failing during model-config parsing.

## Changes

- Updated Q8_0, Q5_0, Q4_K, and Q6_K scalar Llama fixtures from
  `llama.rope.dimension_count = 0` to `2`.
- Kept tensor shapes and quantized tensor payloads unchanged.

## Evidence

The failing command was:

```sh
cargo test -p ferrite-inference -- --nocapture
```

It failed in quantized fixture loading paths with:

```text
Error: InferenceError { message: "llama.rope.dimension_count must be greater than zero" }
```

## Scope

This is a fixture metadata repair. It does not change real-model parsing,
inference math, or quantized kernel behavior.
