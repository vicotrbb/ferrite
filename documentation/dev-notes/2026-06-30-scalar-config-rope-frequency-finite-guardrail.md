# Scalar Config RoPE Frequency Finite Guardrail

Ferrite now rejects non-finite `ScalarLlamaConfig::rope_freq_base` values when
constructing an in-memory scalar model.

The GGUF parser already rejects non-finite explicit `*.rope.freq_base`
metadata, but direct scalar configs could still bypass that parser-level
guardrail. The scalar validation boundary now enforces the same invariant for
test fixtures, synthetic models, and any future direct construction paths.

## Changes

- Added scalar config validation requiring `rope_freq_base` to be finite.
- Added a regression test covering `NaN` and positive infinity through
  `ScalarLlamaModel::new`.

## Red Test

```sh
cargo test -p ferrite-inference --test scalar_reference scalar_config_rejects_non_finite_rope_freq_base -- --nocapture
```

Failed before implementation with:

```text
Error: Custom { kind: Other, error: "non-finite rope frequency base should be rejected" }
test scalar_config_rejects_non_finite_rope_freq_base ... FAILED
```

## Green Test

```sh
cargo test -p ferrite-inference --test scalar_reference scalar_config_rejects_non_finite_rope_freq_base -- --nocapture
```

Passed after implementation:

```text
test scalar_config_rejects_non_finite_rope_freq_base ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 22 filtered out
```

## Scope

This is a focused scalar validation guardrail. It does not change RoPE
arithmetic, GGUF parsing, or model loading behavior for finite positive
frequency bases.
