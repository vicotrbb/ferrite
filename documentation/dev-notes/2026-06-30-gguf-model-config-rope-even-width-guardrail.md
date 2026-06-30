# GGUF Model Config RoPE Even Width Guardrail

Ferrite now rejects odd `*.rope.dimension_count` metadata while deriving a
GGUF model config.

RoPE rotates dimensions in pairs. An odd rotary width would leave an incomplete
pair and already violates the scalar inference validation invariant. Rejecting
it at model-config derivation keeps malformed GGUF metadata from reaching later
loader or execution paths.

## Changes

- Added GGUF model-config validation for
  `{architecture}.rope.dimension_count % 2 == 0`.
- Added a regression test for `llama.rope.dimension_count = 3`.
- Adjusted the existing over-wide RoPE fixture from `5` to `6` so it remains
  even and continues to exercise only the key-width guardrail.

## Red Test

```sh
cargo test -p ferrite-model --test gguf_config rejects_odd_rope_dimension_count -- --nocapture
```

Failed before implementation with:

```text
Error: Custom { kind: Other, error: "odd rope dimension count should be rejected" }
test rejects_odd_rope_dimension_count ... FAILED
```

## Green Test

```sh
cargo test -p ferrite-model --test gguf_config rejects_odd_rope_dimension_count -- --nocapture
```

Passed after implementation:

```text
test rejects_odd_rope_dimension_count ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 15 filtered out
```

## Scope

This is a focused metadata guardrail. It does not change RoPE arithmetic,
architecture support, or tensor loading behavior.
