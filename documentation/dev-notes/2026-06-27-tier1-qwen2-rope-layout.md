# 2026-06-27 Tier 1 Qwen2 RoPE Layout

## Scope

This slice adds an explicit scalar RoPE layout option and maps Qwen2 to the
split-half layout used by `llama.cpp`.

It fixes the Qwen2.5-0.5B deterministic reference mismatch recorded in the
loader-dispatch slice.

## Evidence Before Change

Ferrite could load and execute Qwen2.5-0.5B-Instruct Q4_K_M, but the three-token
reference check failed:

```text
generated_token_ids=2857,25,16
expected_generated_token_ids=198,9707,11
generated_match=false
```

Local `llama.cpp` source inspection showed Qwen2 uses the RoPE layout where
head-value pairs are offset by `n_rot / 2`, not adjacent pairs.

## Implementation

- Added `RopeLayout::AdjacentPairs` and `RopeLayout::SplitHalf`.
- Kept public `apply_rope()` as the existing adjacent-pair helper.
- Added a layout-aware internal RoPE helper for scalar sessions.
- Set Llama loader configs to `AdjacentPairs`.
- Set Qwen2 loader configs to `SplitHalf`.
- Added a unit test for split-half rotation behavior.

## Validation

Commands:

```sh
cargo fmt --all -- --check
cargo test -p ferrite-inference --test scalar_reference -- --nocapture
cargo test -p ferrite-inference rope_split_half_layout_rotates_values_offset_by_half_dimension -- --nocapture
cargo clippy -p ferrite-inference --all-targets -- -D warnings
cargo run --release -p ferrite-cli -- --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --prompt 'hello world' --generate-tokens 3 --expect-generated-token-ids 198,9707,11
```

All commands passed.

The real Qwen2.5-0.5B parity run produced:

```text
prompt_token_ids=14990,1879
next_token_id=198
next_token=Ċ
generated_cached_tokens=5
generated_token_ids=198,9707,11
generated_text=
Hello,
expected_generated_token_ids=198,9707,11
generated_match=true
model_file_bytes=397808192
model_file_retained_bytes=0
scalar_weight_bytes=391859712
kv_cache_bytes=122880
```

## Result

Ferrite now has a deterministic reference-token proof for the real Tier 1
Qwen2.5-0.5B-Instruct Q4_K_M artifact over three generated tokens from the
prompt `hello world`.

This proves one small Qwen2 Tier 1 output path. It does not prove Qwen2.5-1.5B,
head_dim=128 Qwen2 behavior, throughput, or broader Qwen2 prompt coverage.
