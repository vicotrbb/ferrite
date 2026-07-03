# Theory: Qwen 0.5B Required-Gate Cache Ladder

Date: 2026-07-03

Status: Validated locally for Qwen2.5-0.5B Q4_K_M; Testing broader models

## Hypothesis

The local Qwen2.5-0.5B long-chat ladder is dominated less by raw 1024-token
decode throughput than by prompt-cache depth on generated follow-up turns.
When generated context reaches a prompt fixed point, TTFT collapses even at a
1024-token completion budget.

## Mechanism

The hardened long-chat gate now proves the model set, token-length set, and
probe set required for each run. The 1024-token local Qwen slice used:

- `--require-models qwen2.5-0.5b-q4_k_m`
- `--require-token-lengths 1024`
- `--require-probes error,disconnect`
- `--prompt-cache-trace`
- generated assistant context carried across four turns

Turns 2 and 3 reused only a shallow shared prefix, so they still paid a large
prefill cost. Turn 4 reused the full rendered prompt and skipped that cost.

## Evidence

Benchmark note:

`documentation/benchmarks/2026-07-03-local-qwen-0-5b-long-chat-required-gates-1024.md`

Proof log:

`target/proof/local-qwen05-long-chat-required-gates-1024-2026-07-03/long-chat.log`

The run completed:

```text
long_chat_summary_required_models_present=true
long_chat_summary_required_token_lengths_present=true
long_chat_summary_required_probes_completed=true
long_chat_summary_all_follow_up_turns_use_generated_context=true
long_chat_summary_all_generated_context_identities_match_previous_response=true
long_chat_summary_run_complete=true
```

The latency/cache shape was:

| Turn | Context | Cached / Prompt | Lookup | TTFT ms | Stream tok/s |
| ---: | --- | ---: | --- | ---: | ---: |
| 1 | seed | 43 / 43 | `exact_hit` | 3 | 14.446161 |
| 2 | generated | 12 / 1054 | `shared_prefix_hit` | 65919 | 5.593656 |
| 3 | generated | 16 / 1054 | `shared_prefix_hit` | 64108 | 5.461355 |
| 4 | generated | 1054 / 1054 | `exact_hit` | 96 | 8.904586 |

Turn 3 generated the same response identity it received as assistant context:

```text
long_chat_result_assistant_context_hash=fnv64:d3b6392e4ebce4da
long_chat_result_generated_response_hash=fnv64:d3b6392e4ebce4da
```

Turn 4 then reused that same generated context and reported an exact prompt
hit. This validates the fixed-point cache mechanism under the hardened
required-gate path.

## Expected Measurement

For any other model or token-length lane, this theory predicts:

- shallow shared-prefix hits will keep TTFT high even when decode throughput is
  acceptable;
- full prompt reuse will reduce TTFT by orders of magnitude;
- generated-response identity links should explain exact-hit transitions;
- RSS should stay bounded if cache restoration does not leak per-turn state.

## Falsification Experiment

Repeat the required-gate shape on the remaining Tier 1 models:

- `Qwen2.5-1.5B-Instruct-Q8_0`
- `Qwen2.5-1.5B-Instruct-Q6_K`
- `SmolLM2-1.7B-Instruct-Q4_K_M`

The theory is weakened if exact prompt hits do not materially reduce TTFT, or
if shallow prefix reuse produces low TTFT without another clear lifecycle
explanation. It is falsified if generated-context identity links are wrong but
exact prompt-cache hits still appear as valid.

## Risks

- Exact prompt reuse may indicate semantic repetition, not a better chat UX.
- Optimizing only for fixed points could make real changing conversations no
  faster.
- The Qwen2.5-0.5B result may not transfer to larger Qwen artifacts or SmolLM2.
- The current local proof covered `error,disconnect`; queue behavior and
  stop/EOS behavior remain outside this result.

## Next Step

Run a targeted queue-probe slice and a stop/EOS slice before broadening model
coverage. That keeps the gate honest: cache speedups should not hide missing
operational behavior.
