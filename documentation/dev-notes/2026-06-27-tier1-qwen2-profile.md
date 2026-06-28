# 2026-06-27 Tier 1 Qwen2 Profile

## Scope

This slice records `--profile-next-token` output summaries for the Tier 1
Qwen2 Q4_K_M models after deterministic reference parity was proven.

It is profiling evidence only. It does not change runtime code and does not
make a throughput claim.

## Commands

```sh
target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --prompt 'hello world' --profile-next-token
target/release/ferrite --model target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf --prompt 'hello world' --profile-next-token
```

Both commands exited successfully and produced `next_token_id=198`.

## Qwen2.5-0.5B-Instruct Q4_K_M

Summary:

```text
profile_next_token_total_ns=176875253
profile_next_token_role=ffn_down:Q4_K:896:4864:2451456:5455999
profile_next_token_role=ffn_down:Q6_K:896:4864:3575040:7355042
profile_next_token_role=ffn_gate:Q5_0:4864:896:2996224:62354126
profile_next_token_role=ffn_up:Q5_0:4864:896:2996224:61310126
profile_next_token_role=k_proj:Q5_0:128:896:78848:1701043
profile_next_token_role=o_proj:Q5_0:896:896:551936:11646249
profile_next_token_role=output:Q8_0:151936:896:144643072:14057292
profile_next_token_role=q_proj:Q5_0:896:896:551936:11887834
profile_next_token_role=v_proj:Q5_0:128:896:78848:816461
profile_next_token_role=v_proj:Q8_0:128:896:121856:291081
```

The 0.5B profile is dominated by Q5_0 FFN gate/up projections. Together, those
two role summaries account for 123,664,252 ns of the profiled next-token time.

## Qwen2.5-1.5B-Instruct Q4_K_M

Summary:

```text
profile_next_token_total_ns=102538405
profile_next_token_role=ffn_down:Q4_K:1536:8960:7741440:10449291
profile_next_token_role=ffn_down:Q6_K:1536:8960:11289600:13335916
profile_next_token_role=ffn_gate:Q4_K:8960:1536:7741440:22028789
profile_next_token_role=ffn_up:Q4_K:8960:1536:7741440:22151416
profile_next_token_role=k_proj:Q4_K:256:1536:221184:2600458
profile_next_token_role=o_proj:Q4_K:1536:1536:1327104:6100039
profile_next_token_role=output:Q6_K:151936:1536:191439360:16894125
profile_next_token_role=q_proj:Q4_K:1536:1536:1327104:6248873
profile_next_token_role=v_proj:Q4_K:256:1536:221184:1230501
profile_next_token_role=v_proj:Q6_K:256:1536:322560:1498997
```

The 1.5B profile is dominated by Q4_K FFN gate/up projections, the Q6_K output
projection, and Q4_K/Q6_K FFN down projections. This differs from the 0.5B
profile, where Q5_0 FFN gate/up dominate.

## Interpretation

The next Qwen2 optimization should not assume the Q4_K/Q6_K-only profile from
SmolLM2 generalizes to Qwen2.5-0.5B. That model's hot path is Q5_0-heavy.

For Qwen2.5-1.5B, the existing Q4_K and Q6_K fused NEON paths are already on
the hot formats, but throughput is still below target. The next useful
experiments are likely thresholded scheduling, output-projection-specific
argmax work, or Q5_0 block-dot improvements, each with a benchmark gate.
