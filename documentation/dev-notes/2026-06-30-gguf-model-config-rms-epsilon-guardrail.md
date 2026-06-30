# GGUF Model Config RMS Epsilon Guardrail

Ferrite now rejects invalid explicit
`*.attention.layer_norm_rms_epsilon` metadata while deriving a GGUF model
config.

The scalar RMS norm path adds epsilon before taking a square root. Negative,
`NaN`, or infinite metadata can produce non-finite activations or undefined
normalization behavior. Rejecting invalid epsilon metadata at config parsing
keeps malformed GGUF files from reaching the scalar loader or execution paths.

## Changes

- Added GGUF model-config validation requiring explicit
  `{architecture}.attention.layer_norm_rms_epsilon` to be finite and
  non-negative.
- Added a fixture helper for varying Llama RMS epsilon metadata.
- Added a regression test covering `-1.0`, `NaN`, and positive infinity.

## Red Test

```sh
cargo test -p ferrite-model --test gguf_config rejects_invalid_attention_layer_norm_rms_epsilon -- --nocapture
```

Failed before implementation with:

```text
Error: Custom { kind: Other, error: "invalid attention layer norm RMS epsilon should be rejected" }
test rejects_invalid_attention_layer_norm_rms_epsilon ... FAILED
```

## Green Test

```sh
cargo test -p ferrite-model --test gguf_config rejects_invalid_attention_layer_norm_rms_epsilon -- --nocapture
```

Passed after implementation:

```text
test rejects_invalid_attention_layer_norm_rms_epsilon ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 18 filtered out
```

## Scope

This is a focused metadata guardrail. It does not change RMS norm arithmetic,
default epsilon behavior when metadata is omitted, or tensor loading.
