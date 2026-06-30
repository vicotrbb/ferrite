# Scalar Config RMS Epsilon Guardrail

Ferrite now rejects invalid `ScalarLlamaConfig::rms_norm_epsilon` values when
constructing an in-memory scalar model.

The GGUF parser already rejects invalid explicit
`*.attention.layer_norm_rms_epsilon` metadata, but direct scalar configs could
still bypass that parser-level guardrail. The scalar validation boundary now
enforces the same finite, non-negative invariant for test fixtures, synthetic
models, and future direct construction paths.

## Changes

- Added scalar config validation requiring `rms_norm_epsilon` to be finite and
  non-negative.
- Added a regression test covering `-1.0`, `NaN`, and positive infinity through
  `ScalarLlamaModel::new`.

## Red Test

```sh
cargo test -p ferrite-inference --test scalar_reference scalar_config_rejects_invalid_rms_norm_epsilon -- --nocapture
```

Failed before implementation with:

```text
Error: Custom { kind: Other, error: "invalid RMS norm epsilon should be rejected" }
test scalar_config_rejects_invalid_rms_norm_epsilon ... FAILED
```

## Green Test

```sh
cargo test -p ferrite-inference --test scalar_reference scalar_config_rejects_invalid_rms_norm_epsilon -- --nocapture
```

Passed after implementation:

```text
test scalar_config_rejects_invalid_rms_norm_epsilon ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 23 filtered out
```

## Scope

This is a focused scalar validation guardrail. It does not change RMS norm
arithmetic, GGUF parsing, or model loading behavior for finite non-negative
epsilon values.
