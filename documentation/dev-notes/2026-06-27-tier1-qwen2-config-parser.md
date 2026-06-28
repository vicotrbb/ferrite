# 2026-06-27 Tier 1 Qwen2 Config Parser

## Scope

This slice adds architecture-aware GGUF model config parsing for Llama and
Qwen2 metadata.

It does not add Qwen2 inference execution. The scalar runtime still calls the
Llama-only loader path, so real Qwen2 GGUF execution remains unsupported.

## Implementation

- Added `ModelArchitecture` and `ModelConfig` as public GGUF config types.
- Moved the shared transformer config shape into a focused model config module.
- Added `GgufFile::model_config()` for architecture-aware config extraction.
- Kept `GgufFile::llama_config()` as the existing compatibility boundary for
  the scalar Llama loader.
- Added synthetic GGUF parser tests for:
  - Llama config extraction through `model_config()`;
  - Qwen2 config extraction from `qwen2.*` metadata;
  - Qwen2 7:1 GQA ratio derivation.

## Validation

Commands:

```sh
cargo test -p ferrite-model --test gguf_reader -- --nocapture
cargo test -p ferrite-inference --test scalar_reference -- --nocapture
cargo test -p ferrite-inference --test scalar_session_cache -- --nocapture
```

All commands passed.

The Qwen2 fixture validates:

```text
attention_head_count=14
attention_head_count_kv=2
key_length=64
value_length=64
rope_dimension_count=64
gqa_ratio=7
```

## Result

Ferrite can now parse Qwen2 Tier 1 model config metadata at the GGUF model
crate boundary. This removes the first metadata-only blocker identified by the
Qwen2.5-0.5B probe.

Remaining Qwen2 work is runtime-facing: scalar loader architecture dispatch,
attention bias handling or validation, deterministic reference-token parity,
and benchmark evidence before any Qwen2 support claim.
