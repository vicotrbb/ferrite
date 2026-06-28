# Q8_K Single-Role Probes

Date: 2026-06-28

## Scope

This note follows `documentation/dev-notes/2026-06-28-q8-k-role-scope-probes.md`
by testing single-role Path B scopes on SmolLM2-1.7B-Instruct Q4_K_M.

The question was whether any individual projection role is parity-safe on the
fixed prompts and whether the known `hello world` divergent prefix still crosses
the same narrow top-logit margin.

## Generation Probes

All commands used the rebuilt release CLI:

```sh
cargo build --release -p ferrite-cli
```

| Role scope | `hello world` generated IDs | `The capital of France is` generated IDs | Result |
| --- | --- | --- | --- |
| `output` | `18,198,198,3272,24,2334` | `7042,30,2` | Prompt-sensitive divergence |
| `ffn_down` | `18,198,198,3272,24,2334` | `7042,30,2` | Prompt-sensitive divergence |
| `ffn_gate` | `18,198,198,19,21367,42` | `7042,30,378,3575,282,4649` | Diverged on both prompts |
| `ffn_up` | `18,198,3725,198,198,788` | `7042,30,2` | Matched both fixed prompts |

The default references for these prompts remain:

```text
hello world -> 18,198,3725,198,198,788
The capital of France is -> 7042,30,2
```

## Divergent Prefix Profile

The known `hello world` divergent prefix is:

```text
28120,905,18,198
```

Filtered `--profile-next-token --compare-q8-k-activation-matvec --top-logits 3`
results:

| Role scope | Next token | Top logits | Max absolute comparison diff |
| --- | ---: | --- | ---: |
| `output` | `198` | `198:15.956839,3725:15.944314,3272:14.680927` | `0.953757` |
| `ffn_down` | `198` | `198:16.217583,3725:16.161804,3272:15.164220` | `3.876663` |
| `ffn_gate` | `198` | `198:16.231976,3725:15.959627,3272:14.503354` | `0.614862` |
| `ffn_up` | `3725` | `3725:16.122206,198:15.967596,3272:14.940659` | `0.194223` |

For multi-layer roles, the max absolute diff above is the largest observed
`profile_next_token_q8_k_compare` value across that role's layer comparisons.

## Local Benchmark Signal

Same-binary `hello world` benchmark runs:

| Policy | benchmark_token_ids | benchmark_avg_ns |
| --- | --- | ---: |
| default | `198,3725,198,198,788` | `310,829,191` |
| `ffn_up` only | `198,3725,198,198,788` | `296,535,858` |

This is a small local signal, not a promotion gate. It shows that a narrow
`ffn_up`-only scope can preserve the fixed SmolLM2 prompts and modestly improve
this benchmark run, while other single roles remain prompt-sensitive or unsafe.

## Conclusion

The single-role probes narrow the Path B policy question:

- `output`, `ffn_down`, and `ffn_gate` are not default-safe on this evidence.
- `ffn_up` is the only tested single-role candidate that matched both fixed
  SmolLM2 prompts and preserved the known divergent-prefix decision.
- `ffn_up` still needs broader prompt/model parity and repeated benchmark
  evidence before any dispatch promotion can be considered.

The next evidence slice should test `ffn_up` across the existing six fixed
SmolLM2 prompts and the Qwen2 Tier 1 prompt set, then benchmark repeated runs if
parity holds.
