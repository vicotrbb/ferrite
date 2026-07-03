# Theory: Qwen 1.5B Q8 Cache Stability

Date: 2026-07-03

Status: Testing

## Hypothesis

Qwen2.5-1.5B Q8 long-chat latency is dominated by generated-context prompt
growth unless the prompt cache can reuse a deep shared prefix or exact prompt
identity. At the 256-token budget, generated responses do not naturally converge
to a fixed point, so follow-up TTFT remains high even though every follow-up
turn is technically cached.

## Mechanism

The current long-chat gate carries generated assistant content into the next
turn. For Qwen 1.5B Q8 at 256 generated tokens, that assistant content changes
on every follow-up turn. The rendered prompt therefore changes too, and the
prefix cache can only reuse the stable chat prefix and a small portion of the
prior prompt.

That produces `shared_prefix_hit` rows rather than `exact_hit` rows. Since the
model is larger than Qwen 0.5B, even shallow prefill remains expensive enough to
dominate user-visible time to first token.

## Current Evidence

Current-commit benchmark note:

`documentation/benchmarks/2026-07-03-openai-long-chat-x86-qwen-1-5b-q8-current-256.md`

The run passed on a bounded x86_64 `staging` pod with:

```text
long_chat_summary_all_generated_follow_up_turns_cached=true
long_chat_summary_all_generated_context_identities_match_previous_response=true
long_chat_summary_run_complete=true
```

But cache depth remained shallow:

| Turn | Prompt | Cached | Lookup | Response hash | TTFT ms | Decode tok/s |
| ---: | ---: | ---: | --- | --- | ---: | ---: |
| 2 | 287 | 12 | `shared_prefix_hit` | `fnv64:9bbbc743c206d034` | 67923 | 3.632738 |
| 3 | 287 | 34 | `shared_prefix_hit` | `fnv64:5137dd192dda0ce9` | 62654 | 3.626173 |
| 4 | 282 | 34 | `shared_prefix_hit` | `fnv64:39eb905e38437a75` | 61136 | 3.625616 |

The response hash changed on every generated follow-up turn. That explains why
this lane did not reproduce the Qwen 0.5B 1024 exact-hit collapse.

## Expected Measurement

This theory is strengthened if future Qwen 1.5B Q8 runs show:

- generated-response hashes continue to change across follow-up turns;
- prompt cache lookup remains `shared_prefix_hit`;
- cached prompt tokens stay shallow relative to prompt tokens;
- TTFT remains much higher than seed-turn TTFT;
- decode throughput varies less than TTFT.

It is weakened if a 512 or 1024 lane reaches a stable generated-response
identity and still fails to produce full prompt reuse.

## Falsification Experiment

Run two 256-token Qwen 1.5B Q8 variants:

- fixed-answer prompt that should converge to repeated assistant output;
- open-ended prompt that should keep generated responses changing.

The theory is falsified if both variants show the same cache depth and TTFT
pattern despite clearly different generated-response identity behavior.

## Risks

- A 256-token lane may not represent 512 or 1024 behavior. Longer outputs can
  either stabilize into repeated text or drift further.
- Prompt-cache depth is not the only cost. Decode throughput also drops from
  the seed turn to generated follow-up turns.
- The run used an 8Gi pod limit. It should not be cited as proof that Q8 fits
  a 6Gi serving envelope.

## Next Step

Run Qwen 1.5B Q8 at 512 tokens with the same current lifecycle/cache gate and
fresh prompt-cache key. Compare:

- generated-response identity stability;
- prompt-token count;
- cached prompt tokens;
- TTFT and decode token/sec;
- RSS and cgroup peak.

If 512 also stays shallow, test a proof-only structured state capsule before
trying larger cache-policy changes.
