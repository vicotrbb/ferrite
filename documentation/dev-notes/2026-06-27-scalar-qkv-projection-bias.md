# 2026-06-27 Scalar QKV Projection Bias

## Scope

This slice adds optional Q/K/V projection bias support to the scalar inference
path.

It is a runtime prerequisite for Qwen2.5 GGUF files, which expose
`blk.N.attn_q.bias`, `blk.N.attn_k.bias`, and `blk.N.attn_v.bias` tensors.
It does not claim Qwen2 model execution yet because the scalar loader still
uses the Llama-only config boundary.

## Implementation

- Added optional `q_bias`, `k_bias`, and `v_bias` vectors to
  `ScalarLlamaLayerWeights`.
- Validated optional bias lengths against hidden width or KV width.
- Loaded optional GGUF bias tensors when present.
- Applied Q/K/V biases immediately after projection matvecs and before RoPE.
- Included optional bias vectors in scalar weight memory accounting.
- Added a scalar reference test proving a V projection bias contributes to the
  hidden state and changes the selected token.

## Validation

Commands:

```sh
cargo fmt --all -- --check
cargo test -p ferrite-inference --test scalar_reference -- --nocapture
cargo test -p ferrite-inference --test scalar_session_cache -- --nocapture
cargo clippy -p ferrite-inference --all-targets -- -D warnings
```

All commands passed.

## Result

The scalar runtime can now represent, validate, load, account for, and apply
optional Q/K/V projection biases. Qwen2 execution still needs architecture
dispatch from parsed `ModelConfig::Qwen2` into a runtime loader plus reference
token parity before support can be claimed.
