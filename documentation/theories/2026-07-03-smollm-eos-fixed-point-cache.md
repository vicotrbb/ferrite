# Theory: SmolLM2 EOS Fixed-Point Cache

Date: 2026-07-03

Status: Testing

## Hypothesis

Natural-EOS conversations with tiny generated assistant payloads can reach an
exact prompt fixed point faster than long token-limit conversations. When the
assistant response stabilizes after EOS, the next turn's rendered prompt token
identity can exactly match the previous turn and collapse TTFT to milliseconds.

## Mechanism

The SmolLM2 EOS gate uses a stable follow-up prompt and carries generated
assistant content into the next request. If the model repeatedly emits the same
short assistant content before EOS, the generated assistant context becomes
stable. With a stable model id, prompt renderer, follow-up text, assistant
context, and prompt-cache key, the tokenized prompt for the next turn can become
identical to the cached previous prompt.

That is different from a long token-limit lane where hundreds or thousands of
generated tokens can drift between turns and prevent full prompt identity.

## Current Evidence

The local lifecycle SmolLM2 1.7B Q4_K_M gate and the x86_64 staging rerun both
showed the same qualitative pattern.

Local proof:

`documentation/benchmarks/2026-07-03-local-smollm-1-7b-lifecycle-long-chat-eos-gate.md`

| Turn | Cached / Prompt | Lookup | TTFT ms | Finish |
| ---: | ---: | --- | ---: | --- |
| 1 | 0 / 48 | `miss` | 7969 | `stop` |
| 2 | 22 / 46 | `shared_prefix_hit` | 4067 | `stop` |
| 3 | 46 / 46 | `exact_hit` | 32 | `stop` |
| 4 | 46 / 46 | `exact_hit` | 31 | `stop` |

x86_64 proof:

`documentation/benchmarks/2026-07-03-openai-long-chat-x86-smollm-1-7b-lifecycle-eos-gate.md`

| Turn | Cached / Prompt | Lookup | TTFT ms | Finish |
| ---: | ---: | --- | ---: | --- |
| 1 | 0 / 48 | `miss` | 39722 | `stop` |
| 2 | 22 / 46 | `shared_prefix_hit` | 20086 | `stop` |
| 3 | 46 / 46 | `exact_hit` | 35 | `stop` |
| 4 | 46 / 46 | `exact_hit` | 31 | `stop` |

Both runs also reported:

```text
long_chat_summary_all_generated_context_identities_match_previous_response=true
long_chat_summary_all_generated_follow_up_turns_cached=true
long_chat_summary_run_complete=true
```

## Expected Measurement

This theory is strengthened when a natural-EOS lane shows:

- repeated generated-response identity across turns;
- next-turn assistant-context identity matching the previous response;
- `cached_prompt_tokens == prompt_tokens`;
- `prompt_cache_lookup=exact_hit`;
- TTFT near tens of milliseconds even when cache misses take seconds or
  minutes on the same machine.

## Falsification Experiment

Run the same SmolLM2 EOS gate with a follow-up prompt that forces semantically
different short answers on every turn. The theory is weakened if exact hits
still appear without stable generated-response identity. It is falsified if
stable generated-response identity and stable prompt token hashes do not lead to
exact hits under the same cache namespace.

## Risks

- A stable one-token answer is useful for cache mechanism proof, but it can
  overstate real chat latency where answers change each turn.
- Exact-hit optimization may reward repetition, so correctness gates must keep
  generated-context identity separate from semantic quality.
- The x86_64 run was resource-bounded and slow on cache misses; absolute TTFT
  should not be generalized to production hardware.

## Next Step

Add a small EOS prompt set with two lanes:

- fixed-answer lane, expected to converge to exact hits;
- changing-answer lane, expected to stay in shared-prefix or miss territory.

The acceptance criterion is not lower latency alone. The proof must explain
latency through generated-response identity, prompt token hash identity, cached
prompt tokens, and lifecycle completion.
