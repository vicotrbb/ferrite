# Theory: Long-Chat Prefix Reuse

Date: 2026-07-02

Status: Testing

## Hypothesis

Ferrite's generated-context long-chat slowdown is dominated by reprocessing the
same conversation prefix on every follow-up request. Reusing the KV cache for
the unchanged prefix across repeated chat turns should reduce time to first
token for turns 2 and later without changing generated text, usage accounting,
or OpenAI-compatible response shape.

## Mechanism

The Qwen2.5-1.5B Q8_0 x86_64 generated-context proof shows a sharp TTFT jump
when the prompt grows from the seed turn to generated-context follow-up turns:

| Max tokens | Turn | Context | Prompt tokens | TTFT ms | Tok/s | RSS before |
| ---: | ---: | --- | ---: | ---: | ---: | ---: |
| 256 | 1 | seed | 43 | 9930 | 3.391947 | 1940291584 |
| 256 | 2 | generated | 287 | 72951 | 1.807133 | 1940291584 |
| 512 | 1 | seed | 43 | 9936 | 3.621099 | 1956052992 |
| 512 | 2 | generated | 553 | 142022 | 1.696428 | 1956052992 |
| 1024 | 1 | seed | 43 | 10037 | 3.387412 | 1995456512 |
| 1024 | 2 | generated | 1080 | 309182 | 1.391519 | 1987674112 |

The follow-up turns preserve the previous assistant output as generated context,
so a large prefix is identical between adjacent turns. If Ferrite recomputes the
entire prompt for each request, TTFT scales with total prompt length. A
session-level prefix cache could avoid recomputing K/V states for the stable
prefix and only evaluate the newly appended user/assistant suffix.

## Expected Measurement

This theory is worth pursuing if a measurement-only probe confirms that TTFT is
approximately proportional to prompt-token growth while per-token decode latency
stays in a narrower range. For the current Q8_0 1024 proof, the first useful
target is reducing generated follow-up TTFT by at least 30 percent without
increasing RSS by more than 10 percent after idle.

The first implementation-worthy proof would show:

- same prompt-token and completion-token accounting as the baseline;
- same `finish_reason` behavior for length-limited runs;
- same generated-context source reporting for turns 2-4;
- lower TTFT for turns 2-4 on a repeated generated-context run;
- bounded RSS after idle, with no persistent growth across repeated sessions.

## Falsification Experiment

Before implementing a cache, run a measurement-only baseline that separates
prefill time from decode time for seed and generated-context turns. This can be
done by extending or wrapping the existing long-chat gate to record:

- prompt-token count;
- prefill elapsed time before first streamed token;
- first-token timestamp;
- decode elapsed time after first token;
- RSS before request, after request, and after idle.

The theory is falsified for the current milestone if generated follow-up TTFT is
not materially tied to prompt-token growth, or if a prototype prefix cache
reduces TTFT by less than 10 percent while adding meaningful memory retention or
complex session invalidation risk.

## Risks

- Prefix identity is subtle for chat templates, system messages, tool fields,
  stop sequences, sampling settings, and tokenizer control tokens.
- OpenAI-compatible clients do not expose a stable session identifier by
  default, so server-side cache keys must be explicit and conservative.
- KV reuse can silently corrupt generation if the cached prefix is matched at
  the string level instead of token level.
- Keeping K/V states across requests may hurt memory-fit goals unless eviction
  is strict and observable.

## Instrumentation Progress

The long-chat gate now emits stream-observed timing split fields:

- `long_chat_result_stream_observed_prefill_elapsed_ms`
- `long_chat_result_first_token_timestamp_ms`
- `long_chat_result_stream_observed_decode_elapsed_ms`
- `long_chat_result_stream_observed_decode_tokens_per_second`

These are derived from client-observed SSE token event offsets. They are useful
for comparing first-token delay against post-first-token decode pace, but they
do not expose internal engine prefill timing directly.

## Timing Theory Probes

The first live x86_64 Qwen2.5-1.5B Q8_0 generated-context rerun with the timing
split completed on 2026-07-02. The benchmark note is
`documentation/benchmarks/2026-07-02-openai-long-chat-x86-qwen-1-5b-q8-prefill-decode-theory-256.md`.

The seed turn used 43 prompt tokens and reported 9972 ms stream-observed
prefill. Generated-context turns used 282-287 prompt tokens and reported
69271-70787 ms stream-observed prefill. Decode also slowed, but less sharply:
seed decode was 63488 ms at 4.032249 decode token events/sec, while
generated-context decode averaged about 71463 ms at about 3.58 decode token
events/sec.

This supports prefix reuse as a worthwhile next design slice. It does not prove
that prefix reuse alone will recover all throughput, because decode pace also
degraded on generated-context turns.

A 512-token rerun also completed on 2026-07-02. The benchmark note is
`documentation/benchmarks/2026-07-02-openai-long-chat-x86-qwen-1-5b-q8-prefill-decode-theory-512.md`.

The 512-token seed turn used 43 prompt tokens and reported 10003 ms
stream-observed prefill. Generated-context turns used 533-553 prompt tokens and
reported 143512-150282 ms stream-observed prefill. Average generated-context
prefill was about 146094 ms, roughly 14.6x the seed prefill. Generated-context
decode averaged about 170028 ms at about 3.01 decode token events/sec, about 20
percent slower than the seed decode event rate.

A 1024-token rerun completed the initial timing-theory set on 2026-07-02. The
benchmark note is
`documentation/benchmarks/2026-07-02-openai-long-chat-x86-qwen-1-5b-q8-prefill-decode-theory-1024.md`.

The 1024-token seed turn used 43 prompt tokens and reported 10160 ms
stream-observed prefill. Generated-context turns used 1054-1080 prompt tokens
and reported 314029-325320 ms stream-observed prefill. Average
generated-context prefill was about 318932 ms, roughly 31.4x the seed prefill.
Generated-context decode averaged about 464706 ms at about 2.20 decode token
events/sec, about 32 percent slower than the seed decode event rate.

Across 256, 512, and 1024 tokens, generated-context first-token delay scales
much faster than seed first-token delay, while post-first-token decode also
degrades materially. This makes prefix reuse the highest-value first-token
latency experiment, not a complete throughput fix.

## Next Step

Design a small token-prefix identity layer and bounded per-model KV prefix cache
as a separate implementation slice. Keep it behind an explicit opt-in or
internal experiment flag until repeated 256/512/1024-token generated-context
proofs show lower first-token latency without RSS drift or response-shape
regression. Track decode slowdown as a separate theory instead of assuming
prefix reuse will fix it.
