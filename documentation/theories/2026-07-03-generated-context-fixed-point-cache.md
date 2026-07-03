# Theory: Generated-Context Fixed-Point Cache Reuse

Date: 2026-07-03

Status: Instrumented hypothesis

## Hypothesis

Some long-chat generated-context lanes converge into a prompt fixed point: a
generated assistant response becomes identical, or token-equivalent after
prompt rendering, to the previous generated assistant response. When that
happens, the next turn's rendered prompt can exactly match the cached previous
turn, producing full prompt-cache reuse and a millisecond-scale TTFT.

## Mechanism

The long-chat gate carries generated assistant context per `(model,
token_length)`. After each scenario, it records the streamed assistant text for
that lane and uses it as the assistant message in the next turn. The user
follow-up stays stable unless state-capsule options decorate it.

Therefore, if the prompt renderer, follow-up text, model id, cache namespace,
and generated assistant context are all unchanged between two turns, the
tokenized prompt identity should be unchanged too. The runtime prefix cache can
then report an `exact_hit` with all prompt tokens cached.

The x86 Qwen 0.5B 1024-token trace supports this mechanism:

| Turn | Prompt hash | Selected entry hash | Cached / Prompt | Lookup | TTFT ms |
| ---: | --- | --- | ---: | --- | ---: |
| 2 | `fnv64:93e2cf81835f98a6` | `fnv64:92585af239e73208` | 12 / 1054 | `shared_prefix_hit` | 378227 |
| 3 | `fnv64:2249cfc489e572a7` | `fnv64:93e2cf81835f98a6` | 16 / 1054 | `shared_prefix_hit` | 374869 |
| 4 | `fnv64:2249cfc489e572a7` | `fnv64:2249cfc489e572a7` | 1054 / 1054 | `exact_hit` | 308 |

The strongest signal is turn 4: its prompt hash equals the selected entry hash,
so Ferrite saw the same prompt token identity as the cached turn 3 prompt.

## Expected Measurement

If this theory is true, a response-context trace should show:

- turn `N` generated response hash equals turn `N-1` generated response hash,
  or their rendered prompt token hashes become equal after normalization;
- the next turn's prompt cache lookup changes from `shared_prefix_hit` to
  `exact_hit`;
- `cached_prompt_tokens == prompt_tokens`;
- TTFT collapses while decode throughput remains in the same broad range.

## Falsification Experiment

Add opt-in generated-response identity output to the long-chat proof tool,
without exposing full generated text by default:

- generated assistant byte length;
- generated assistant chunk count;
- generated assistant FNV64 text hash;
- generated assistant token-id count and token-id hash when token IDs are
  available;
- next-turn assistant-context hash before request construction.

Then rerun the bounded 1024-token Qwen 0.5B trace. The theory is weakened if
turn 3 and turn 4 have different generated-response identity but still produce
the same prompt hash. It is falsified if full-prompt reuse happens without a
stable rendered prompt token identity.

## Risks

- The apparent fixed point may be an artifact of prompt truncation or
  token-windowing in a later run. The current x86 trace did not use generated
  context windowing, so that risk is low for this specific proof.
- Hash equality is not the same as text equality, although the FNV64 collision
  risk is acceptable for diagnostics. A future proof can add length and token
  count to reduce ambiguity.
- Exact prompt reuse may be desirable for latency but can also indicate that a
  chat loop is repeating itself rather than progressing semantically.
- Optimizing for this case alone could hide the main problem: turns 2 and 3
  still reused only 12 to 16 prompt tokens and spent more than six minutes in
  prefill.

## Next Step

The first instrumentation slice is implemented in the proof tooling:

- `StreamingTextSummary` exposes deterministic text identity accessors;
- `LongChatScenarioResult` prints generated-response identity;
- the long-chat runner records the exact assistant-context identity used for
  each request before dispatch;
- fixture tests validate the formatter and generated-context carry path.

Next, rerun the bounded 1024-token Qwen 0.5B trace with prompt-cache tracing
enabled and compare generated-response hashes, next-turn assistant-context
hashes, prompt token hashes, cached prompt tokens, and TTFT.

## First Instrumented Observation

A bounded local 128-token Qwen 0.5B trace confirmed that response/context
identity observability works and that generated responses are carried into the
next request:

| Link | Response hash | Next context hash | Result |
| --- | --- | --- | --- |
| turn 1 -> turn 2 | `fnv64:d6e2f2c865e49919` | `fnv64:d6e2f2c865e49919` | match |
| turn 2 -> turn 3 | `fnv64:0969ba966218802c` | `fnv64:0969ba966218802c` | match |
| turn 3 -> turn 4 | `fnv64:a449d2a8d7a2519c` | `fnv64:a449d2a8d7a2519c` | match |

That run did not show a fixed point: every generated response hash changed,
generated follow-up turns reused only 12, 13, and 14 prompt tokens, and all
generated follow-ups reported `shared_prefix_hit`, not `exact_hit`.
