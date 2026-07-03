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

## Finish-Source Observation

A current-tree rerun now proves that the stable short-answer lane is actually
terminating through tokenizer EOS, not an explicit stop sequence:

`documentation/benchmarks/2026-07-03-local-smollm-1-7b-eos-finish-source-16.md`

| Turn | Cached / Prompt | Finish source | TTFT ms | Context identity |
| ---: | ---: | --- | ---: | --- |
| 1 | 0 / 48 | `eos` | 7879 | seed context |
| 2 | 22 / 46 | `eos` | 3999 | generated context matched turn 1 response |
| 3 | 46 / 46 | `eos` | 8 | generated context matched turn 2 response |
| 4 | 46 / 46 | `eos` | 11 | generated context matched turn 3 response |

This strengthens the theory because the exact cached-token ratio appears only
after the generated assistant response stabilizes, and the terminal condition
is now explicitly observable as `eos`.

Boundary: this rerun did not request prompt-cache trace fields, so it proves
cached-token counts and finish source but does not add new `prompt_cache_lookup`
evidence beyond the earlier trace-bearing lifecycle runs.

## Trace-Bearing Finish-Source Observation

A follow-up current-tree run removed that boundary by enabling
`--prompt-cache-trace` while still requiring `finish_source=eos`:

`documentation/benchmarks/2026-07-03-local-smollm-1-7b-eos-finish-source-trace-16.md`

| Turn | Cached / Prompt | Lookup | Finish source | TTFT ms |
| ---: | ---: | --- | --- | ---: |
| 1 | 0 / 48 | `miss` | `eos` | 7961 |
| 2 | 22 / 46 | `shared_prefix_hit` | `eos` | 4014 |
| 3 | 46 / 46 | `exact_hit` | `eos` | 6 |
| 4 | 46 / 46 | `exact_hit` | `eos` | 10 |

The generated-response hash stabilized as `fnv64:af63c74c8601c8dd` from turn
1 onward. The prompt hash changed from the seed prompt
`fnv64:29bca34202dc5f0a` to the generated-context prompt
`fnv64:67c5f682ed91f353` on turn 2, then stayed fixed for turns 3 and 4. The
selected cache entry hash matched that generated-context prompt hash on turns
3 and 4.

This is stronger evidence for the theory: stable generated content plus a
stable prompt renderer produced exact prompt-cache hits and millisecond TTFT,
while the terminal condition remained observable as tokenizer EOS.

## Changing Follow-Up Diagnostic

A changing-question rerun used the new `--follow-ups` option to ask about
France, Germany, Italy, and Spain across four turns:

`documentation/benchmarks/2026-07-03-local-smollm-1-7b-changing-followups-cache-diagnostic-16.md`

| Turn | Cached / Prompt | Lookup | Finish source | Prompt hash | Generated hash |
| ---: | ---: | --- | --- | --- | --- |
| 1 | 0 / 48 | `miss` | `eos` | `fnv64:29bca34202dc5f0a` | `fnv64:af63c74c8601c8dd` |
| 2 | 22 / 46 | `shared_prefix_hit` | `eos` | `fnv64:69824ec3212819fa` | `fnv64:d975fd21291d28d9` |
| 3 | 22 / 49 | `shared_prefix_hit` | `length` | `fnv64:ddbd1cc3509d39bd` | `fnv64:07b9f98c303e945b` |
| 4 | 23 / 62 | `shared_prefix_hit` | `eos` | `fnv64:b8403071557d00a6` | `fnv64:21d43b1c9ec8810e` |

The strict all-EOS variant of this experiment failed on turn 3 with
`expected finish_reason stop, got length`, so it does not close an EOS-only
falsification lane. The diagnostic rerun still provides useful cache evidence:
when generated response hashes and prompt hashes changed every turn, the cache
never reached `exact_hit`.

This supports the fixed-point explanation over a weaker prompt-template-only
explanation. The stable fixed-answer lane converged to exact hits; the
changing-answer lane retained only shared-prefix hits under the same server
path and cache-key mechanism.

## Expected Measurement

This theory is strengthened when a natural-EOS lane shows:

- repeated generated-response identity across turns;
- next-turn assistant-context identity matching the previous response;
- `long_chat_result_finish_source=eos`;
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
prompt tokens, `finish_source=eos`, prompt-cache trace fields, and lifecycle
completion.

The long-chat gate now supports per-turn follow-up text with `--follow-ups`.
Use that to run the changing-answer lane without changing the OpenAI server or
runtime:

```text
--prompt 'Question: What is the capital of France? Answer only with the city name.'
--assistant-context 'Paris.'
--follow-ups 'Question: What is the capital of France? Answer only with the city name.,Question: What is the capital of Germany? Answer only with the city name.,Question: What is the capital of Italy? Answer only with the city name.,Question: What is the capital of Spain? Answer only with the city name.'
--expect-finish-reason stop
--require-finish-sources eos
--prompt-cache-trace
```

Current result:

- fixed-answer lane: completed with `finish_source=eos`, exact hits on turns 3
  and 4, and millisecond TTFT;
- changing-answer diagnostic lane: completed four turns, changed generated
  response and prompt hashes every turn after the seed, stayed in
  `shared_prefix_hit`, and never reached `exact_hit`;
- changing-answer strict all-EOS lane: not accepted because turn 3 reached the
  token limit.

Expected falsification signal for the remaining stricter lane:

- fixed-answer lane: generated-response hashes stabilize, prompt hashes
  converge, and turns 3-4 become exact hits;
- changing-answer lane: generated-response hashes change by turn, prompt hashes
  do not converge to the same exact prompt identity, or exact-hit TTFT collapse
  disappears.

If the changing-answer lane still reaches exact hits with changing generated
responses, this theory is too weak and must be revised around prompt-template
stability instead of generated-response identity.
