# GGUF Model Config RoPE Frequency Finite Guardrail

Ferrite now rejects non-finite explicit `*.rope.freq_base` metadata while
deriving a GGUF model config.

The previous positive-value guardrail rejected zero and negative frequency
bases, but IEEE `NaN` and infinity still passed the `<= 0.0` check. RoPE angle
calculation needs a finite frequency base, so non-finite metadata is rejected at
config parsing before it can reach the scalar loader or execution paths.

## Changes

- Added GGUF model-config validation requiring explicit
  `{architecture}.rope.freq_base` to be finite.
- Added a regression test covering `NaN` and positive infinity.

## Red Test

```sh
cargo test -p ferrite-model --test gguf_config rejects_non_finite_rope_freq_base -- --nocapture
```

Failed before implementation with:

```text
Error: Custom { kind: Other, error: "non-finite rope frequency base should be rejected" }
test rejects_non_finite_rope_freq_base ... FAILED
```

## Green Test

```sh
cargo test -p ferrite-model --test gguf_config rejects_non_finite_rope_freq_base -- --nocapture
```

Passed after implementation:

```text
test rejects_non_finite_rope_freq_base ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 17 filtered out
```

## Scope

This is a focused metadata guardrail. It does not change RoPE arithmetic,
default frequency-base behavior when metadata is omitted, or tensor loading.
