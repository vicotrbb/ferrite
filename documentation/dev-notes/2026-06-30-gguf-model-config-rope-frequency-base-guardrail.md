# GGUF Model Config RoPE Frequency Base Guardrail

Ferrite now rejects non-positive explicit `*.rope.freq_base` metadata while
deriving a GGUF model config.

RoPE frequency base is used as the denominator base for rotary angle
calculation. Zero or negative values are not meaningful and already violate the
scalar inference validation invariant. Rejecting the metadata at config parsing
keeps malformed GGUF files from reaching later loader or execution paths.

## Changes

- Added GGUF model-config validation for explicit
  `{architecture}.rope.freq_base > 0`.
- Added a fixture helper for varying Llama RoPE frequency base metadata.
- Added a regression test covering `0.0` and `-1.0`.

## Red Test

```sh
cargo test -p ferrite-model --test gguf_config rejects_non_positive_rope_freq_base -- --nocapture
```

Failed before implementation with:

```text
Error: Custom { kind: Other, error: "non-positive rope frequency base should be rejected" }
test rejects_non_positive_rope_freq_base ... FAILED
```

## Green Test

```sh
cargo test -p ferrite-model --test gguf_config rejects_non_positive_rope_freq_base -- --nocapture
```

Passed after implementation:

```text
test rejects_non_positive_rope_freq_base ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 16 filtered out
```

## Scope

This is a focused metadata guardrail. It does not change RoPE arithmetic,
default frequency-base behavior when metadata is omitted, or tensor loading.
